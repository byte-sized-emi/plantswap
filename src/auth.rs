use axum::{async_trait, extract::Query, http::StatusCode, response::{IntoResponse, Redirect}, routing::get, Router};
use axum_htmx::HxRequest;
use axum_login::{tower_sessions::Session, AuthUser, AuthnBackend, UserId};
use jsonwebtoken::{jwk::JwkSet, DecodingKey, Validation};
use oauth2::{basic::{BasicClient, BasicRequestTokenError}, reqwest::{async_http_client, AsyncHttpClientError}, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, TokenResponse, TokenUrl};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::{backend, config::AppConfig, models::UserSession, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_route))
        .route("/redirect", get(redirect_route))
}

const CSRF_STATE_KEY: &str = "oauth.csrf-state";
const PKCE_VERIFIER_KEY: &str = "oauth.pkce-verifier";
const NEXT_URL_KEY: &str = "oauth.next_url";

#[derive(Deserialize)]
struct NextUrl {
    pub next: Option<String>
}

async fn login_route(auth_session: AuthSession, session: Session, HxRequest(is_htmx): HxRequest, Query(next_url): Query<NextUrl>) -> impl IntoResponse {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate the full authorization URL.
    let (auth_url, csrf_token) = auth_session.backend.oauth2_client
        .authorize_url(CsrfToken::new_random)
        // Set the desired scopes.
        // .add_scope(Scope::new("read".to_string()))
        // .add_scope(Scope::new("write".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    session.insert(CSRF_STATE_KEY, csrf_token.secret())
        .await.unwrap();

    session.insert(PKCE_VERIFIER_KEY, pkce_verifier)
        .await.unwrap();

    if let Some(next) = next_url.next {
        session.insert(NEXT_URL_KEY,  next)
            .await.unwrap();
    }

    info!("Sending user to auth url");

    if is_htmx {
        (
            StatusCode::OK,
            [("Hx-Redirect", auth_url.to_string())]
        )
        .into_response()
    } else {
        Redirect::to(auth_url.as_str()).into_response()
    }
}

#[derive(Debug, Deserialize)]
struct AuthzResp {
    pub state: CsrfToken,
    pub code: String,
}

/// The user gets redirected back here from the auth provider after a successful login.
async fn redirect_route(
    mut auth_session: AuthSession,
    session: Session,
    Query(query_params): Query<AuthzResp>
) -> impl IntoResponse {
    if let Some(user) = auth_session.user {
        debug!("User {:?} was already logged in but accessed redirect route", user.id);
        return Redirect::to("/").into_response();
    }

    let old_state = match session.remove(CSRF_STATE_KEY).await {
        Err(err) => {
            error!(?err, id = ?session.id(), "Couldn't get CSRF_STATE_KEY from session");
            return StatusCode::BAD_REQUEST.into_response();
        }
        Ok(None) => {
            error!(id = ?session.id(), "No CSRF_STATE_KEY in session");
            return StatusCode::BAD_REQUEST.into_response();
        }
        Ok(Some(state)) => state
    };

    let Ok(Some(pkce_verifier)) = session.remove(PKCE_VERIFIER_KEY).await else {
        error!(id = ?session.id(), "Couldn't get PKCE_VERIFIER_KEY from session");
        return StatusCode::BAD_REQUEST.into_response();
    };

    let next_url: Option<String> = session.remove(NEXT_URL_KEY).await.ok().flatten();

    let AuthzResp { state: new_state, code } = query_params;

    let credentials = Credentials {
        code,
        old_state,
        new_state,
        pkce_verifier
    };

    let user = match auth_session.authenticate(credentials).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response()
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if auth_session.login(&user).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if let Some(next_url) = next_url {
        Redirect::to(&next_url).into_response()
    } else {
        Redirect::to("/").into_response()
    }
}

#[derive(Clone)]
pub struct AuthState {
    pub jwk_set: JwkSet,
    pub oauth2_client: BasicClient,
    pub backend: backend::Backend,
}

pub async fn initialize_auth(config: &AppConfig, backend: backend::Backend) -> AuthState {
    let server_url = config.auth_server_url();

    let oauth2_client = BasicClient::new(
        ClientId::new(config.auth_client_id().to_string()),
        None,
        AuthUrl::new(format!("{server_url}/protocol/openid-connect/auth")).unwrap(),
        Some(TokenUrl::new(format!("{server_url}/protocol/openid-connect/token")).unwrap())
    )
    .set_redirect_uri(RedirectUrl::new(format!("{}/auth/redirect", config.base_url())).unwrap());

    let jwk_certs_url = format!("{server_url}/protocol/openid-connect/certs");

    let jwk_set = reqwest::get(jwk_certs_url)
        .await.unwrap()
        .json().await
        .unwrap();

    AuthState { jwk_set, oauth2_client, backend }
}

#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub code: String,
    pub old_state: CsrfToken,
    pub new_state: CsrfToken,
    pub pkce_verifier: PkceCodeVerifier
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Error with HTTP request: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("OAuth2 error: {0}")]
    OAuth2(#[from] BasicRequestTokenError<AsyncHttpClientError>),

    #[error("DB error: {0}")]
    Diesel(#[from] diesel::result::Error),
}

#[async_trait]
impl AuthnBackend for AuthState {
    type User = User;
    type Credentials = Credentials;
    type Error = BackendError;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        // Ensure the CSRF state has not been tampered with.
        if creds.old_state.secret() != creds.new_state.secret() {
            return Ok(None);
        };

        // Process authorization code, expecting a token response back.
        let token_res = self
            .oauth2_client
            .exchange_code(AuthorizationCode::new(creds.code))
            .set_pkce_verifier(creds.pkce_verifier)
            .request_async(async_http_client)
            .await?;

        let bearer = token_res.access_token().secret();

        let user_claims = match check_bearer(&self.jwk_set, bearer) {
            Ok(user_claims) => user_claims,
            Err(err) => {
                debug!("Error while checking JWT bearer: {err}");
                return Ok(None);
            }
        };

        let db_user_session = UserSession {
            id: user_claims.user_id,
            access_token: bearer.clone()
        };

        {
            let mut db_con = self.backend.db.lock().await;
            use diesel::prelude::*;
            use crate::schema::user_sessions;

            db_user_session.insert_into(user_sessions::table)
                .on_conflict(user_sessions::columns::id)
                .do_update()
                .set(user_sessions::columns::access_token.eq(bearer))
                .execute(&mut *db_con)?;
        }

        let user = User {
            id: user_claims.user_id,
            claims: user_claims,
            access_token: bearer.clone(),
        };

        Ok(Some(user))
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        use diesel::prelude::*;
        use crate::schema::user_sessions;

        let mut db_con = self.backend.db.lock().await;

        let user_session = user_sessions::table.find(user_id)
            .select(UserSession::as_select())
            .first(&mut *db_con).optional()?;

        if let Some(user_session) = user_session {
            let UserSession { id, access_token } = user_session;

            match check_bearer(&self.jwk_set, &access_token) {
                Ok(claims) => Ok(Some(User {
                    id,
                    claims,
                    access_token
                })),
                Err(err) => {
                    debug!("Error while checking JWT bearer: {err}");
                    return Ok(None);
                }
            }
        } else {
            Ok(None)
        }
    }
}

pub type AuthSession = axum_login::AuthSession<AuthState>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserClaims {
    #[serde(rename = "sub")]
    pub user_id: Uuid,
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub realm_roles: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    id: Uuid,
    pub claims: UserClaims,
    pub access_token: String,
}

// Here we've implemented `Debug` manually to avoid accidentally logging the
// access token.
impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("claims", &self.claims)
            .field("access_token", &"[redacted]")
            .finish()
    }
}

impl AuthUser for User {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.access_token.as_bytes()
    }
}

pub fn check_bearer(jwk_set: &JwkSet, bearer_token: &str) -> Result<UserClaims, jsonwebtoken::errors::Error> {
    let unverified_header =
        jsonwebtoken::decode_header(bearer_token)?;

    let kid = unverified_header.kid
        .expect("Missing 'kid' claim");

    let jwk = jwk_set
        .find(&kid)
        .expect("Invalid key id");

    let decoding_key = DecodingKey::from_jwk(jwk)?;

    let mut validation = Validation::new(unverified_header.alg);
    validation.set_audience(&["plantswap"]);

    debug!("Trying to verify JWT");
    let verified_header = jsonwebtoken::decode(bearer_token, &decoding_key, &validation)?;

    debug!("Auth successful with claims {:?}", verified_header.claims);

    Ok(verified_header.claims)
}
