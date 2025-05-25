use axum::{
    extract::{Query, Request, State}, http::StatusCode, middleware::Next, response::{IntoResponse, Redirect}, routing::get, Extension, Router
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use axum_htmx::HxRequest;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, jwk::JwkSet};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, EndpointNotSet, EndpointSet, HttpClientError,
    PkceCodeChallenge, RedirectUrl, RefreshToken, RequestTokenError, StandardErrorResponse,
    TokenResponse as _, TokenUrl,
    basic::{BasicClient, BasicErrorResponseType},
};
use serde::{Deserialize, Serialize};
use time::Duration;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{config::AppConfig, AppState};

pub mod layers;

const LOGIN_URL: &str = "/auth/login";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_route))
        .route("/redirect", get(redirect_route))
}

const BEARER_COOKIE_NAME: &str = "bearer";
const REFRESH_COOKIE_NAME: &str = "refresh";

const LOGIN_CSRF_COOKIE: &str = "login_csrf";
const LOGIN_PKCE_COOKIE: &str = "login_pkce";
const LOGIN_NEXT_URL_COOKIE: &str = "login_next_url";

#[derive(Deserialize)]
struct NextUrl {
    #[serde(default)]
    pub next: Option<String>,
}

async fn login_route(
    State(auth_state): State<AuthState>,
    mut jar: CookieJar,
    HxRequest(is_htmx): HxRequest,
    Query(next_url): Query<NextUrl>,
) -> impl IntoResponse {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate the full authorization URL.
    let (auth_url, csrf_token) = auth_state
        .oauth2_client
        .authorize_url(CsrfToken::new_random)
        // Set the desired scopes.
        // .add_scope(Scope::new("read".to_string()))
        // .add_scope(Scope::new("write".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    jar = jar
        .add(new_secure_cookie(
            LOGIN_CSRF_COOKIE,
            csrf_token.secret().clone(),
            Duration::minutes(10),
        ))
        .add(new_secure_cookie(
            LOGIN_PKCE_COOKIE,
            serde_json::to_string(&pkce_verifier).unwrap(),
            Duration::minutes(10),
        ));

    if let Some(next_url) = next_url.next {
        jar = jar.add(Cookie::new(LOGIN_NEXT_URL_COOKIE, next_url));
    }

    info!("Sending user to auth url");

    if is_htmx {
        (StatusCode::OK, [("Hx-Redirect", auth_url.to_string())], jar).into_response()
    } else {
        (jar, Redirect::to(auth_url.as_str())).into_response()
    }
}

fn new_secure_cookie(name: &'static str, value: String, max_age: Duration) -> Cookie<'static> {
    Cookie::build((name, value))
        .http_only(true)
        .secure(true)
        .max_age(max_age)
        .build()
}

#[derive(Debug, Deserialize)]
struct AuthzResp {
    pub state: CsrfToken,
    pub code: String,
}

/// The user gets redirected back here from the auth provider after a successful login.
async fn redirect_route(
    State(auth_state): State<AuthState>,
    mut jar: CookieJar,
    Extension(user): Extension<Option<UserClaims>>,
    Query(query_params): Query<AuthzResp>,
) -> impl IntoResponse {
    if let Some(user) = user {
        debug!(
            "User {:?} was already logged in but accessed redirect route",
            user.user_id
        );
        return Redirect::to("/").into_response();
    }

    let old_state = match jar.get(LOGIN_CSRF_COOKIE) {
        Some(old_state) => old_state.value().to_string(),
        None => {
            error!("User tried accessing redirect route without csrf cookie");
            return (
                StatusCode::BAD_REQUEST,
                format!("Missing {LOGIN_CSRF_COOKIE} cookie"),
            )
                .into_response();
        }
    };

    let pkce_verifier = match jar.get(LOGIN_CSRF_COOKIE) {
        Some(cookie) => match serde_json::from_str(cookie.value()) {
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "Invalid pkce verifier cookie").into_response();
            }
            Ok(cookie) => cookie,
        },
        None => {
            error!("User tried accessing redirect route without csrf cookie");
            return (
                StatusCode::BAD_REQUEST,
                format!("Missing {LOGIN_CSRF_COOKIE} cookie"),
            )
                .into_response();
        }
    };

    let next_url = jar
        .get(LOGIN_NEXT_URL_COOKIE)
        .map(|cookie| cookie.value().to_string());

    jar = jar
        .remove(LOGIN_CSRF_COOKIE)
        .remove(LOGIN_PKCE_COOKIE)
        .remove(LOGIN_NEXT_URL_COOKIE);

    let AuthzResp {
        state: new_state,
        code,
    } = query_params;

    if old_state != *new_state.secret() {
        return (StatusCode::BAD_REQUEST, jar, "csrf state doesn't match").into_response();
    }

    let token_response = auth_state
        .oauth2_client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(&auth_state.http_client)
        .await;

    let token_response = match token_response {
        Err(err) => {
            warn!(?err, "error while exchanging auth code");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                jar,
                "Something went wrong trying to exchange auth code",
            )
                .into_response();
        }
        Ok(token_response) => token_response,
    };

    let bearer = token_response.access_token().secret();
    let refresh_token = token_response.refresh_token().map(|t| t.secret());

    if refresh_token.is_none() {
        warn!("Refresh token is none, activate refresh tokens for better security");
    }

    // FIXME: Make the duration's here either configurable,
    //        or extract them from the bearer/refresh JWT.
    //        This doesn't make a difference to security,
    //        as the tokens is checked on every request, but still.
    jar = jar.add(new_secure_cookie(
        BEARER_COOKIE_NAME,
        bearer.clone(),
        Duration::days(100),
    ));

    if let Some(refresh_token) = refresh_token {
        jar = jar.add(new_secure_cookie(
            REFRESH_COOKIE_NAME,
            refresh_token.clone(),
            Duration::days(365),
        ));
    }

    (jar, Redirect::to(&next_url.unwrap_or("/".to_string()))).into_response()
}

pub type Oauth2Client =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

#[derive(Clone)]
pub struct AuthState {
    pub jwk_set: JwkSet,
    pub oauth2_client: Oauth2Client,
    pub http_client: reqwest::Client,
}

pub async fn initialize_auth(config: &AppConfig) -> AuthState {
    let server_url = config.auth_server_url();

    let oauth2_client = BasicClient::new(ClientId::new(config.auth_client_id().to_string()))
        .set_auth_uri(AuthUrl::new(format!("{server_url}/protocol/openid-connect/auth")).unwrap())
        .set_token_uri(
            TokenUrl::new(format!("{server_url}/protocol/openid-connect/token")).unwrap(),
        )
        .set_redirect_uri(
            RedirectUrl::new(format!("{}/auth/redirect", config.base_url())).unwrap(),
        );

    let jwk_certs_url = format!("{server_url}/protocol/openid-connect/certs");

    let jwk_set = reqwest::get(jwk_certs_url)
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest Client should build");

    AuthState {
        jwk_set,
        oauth2_client,
        http_client,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Error with HTTP request: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("OAuth2 error: {0}")]
    OAuth2(
        #[from]
        RequestTokenError<
            HttpClientError<reqwest::Error>,
            StandardErrorResponse<BasicErrorResponseType>,
        >,
    ),

    #[error("DB error: {0}")]
    Diesel(#[from] diesel::result::Error),
}

pub async fn base(
    State(auth_state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> impl IntoResponse {
    let mut jar = CookieJar::from_headers(request.headers());
    let jwt = jar.get(BEARER_COOKIE_NAME)
        .map(|jwt| jwt.value().to_string());
    let refresh = jar.get(REFRESH_COOKIE_NAME)
        .map(|refresh| refresh.value().to_string());

    // Default context for unauthenticated requests
    let mut user: Option<UserClaims> = None;

    // JWT takes precedence if present
    if let Some(jwt) = jwt {
        match check_bearer(&auth_state.jwk_set, &jwt) {
            Ok(claims) => {
                user = Some(claims);
            }
            Err(_) => {
                // Clear potentially compromised cookies
                jar = jar.remove(BEARER_COOKIE_NAME);
            }
        }
    }

    // Fall back to refresh token if JWT is absent/invalid
    if user.is_none() {
        if let Some(refresh) = refresh {
            let refresh_token_response = auth_state
                .oauth2_client
                .exchange_refresh_token(&RefreshToken::new(refresh))
                .request_async(&auth_state.http_client)
                .await;

            match refresh_token_response {
                Err(err) => error!(?err, "Error while trying to exchange refresh token"),
                Ok(refresh_token_response) => {
                    let access_token = refresh_token_response.access_token();

                    jar = jar.add(new_secure_cookie(
                        BEARER_COOKIE_NAME,
                        access_token.secret().to_string(),
                        Duration::days(100),
                    ));

                    match check_bearer(&auth_state.jwk_set, access_token.secret()) {
                        Ok(claims) => {
                            user = Some(claims);
                        }
                        Err(_) => {
                            // Clear potentially compromised cookies
                            jar = jar.remove(BEARER_COOKIE_NAME).remove(REFRESH_COOKIE_NAME);
                        }
                    }
                }
            }
        }
    }

    // Inject the resolved context into request extensions
    request.extensions_mut().insert(user);

    let response = next.run(request).await;

    // Merge cookie updates with the response
    (jar, response).into_response()
}

/// You can extract this from a request by using
/// `Extension(user_claims): Extension<Option<UserClaims>>`
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

const VALID_ALGORITHMS: &[Algorithm] = &[
    Algorithm::RS256,
    Algorithm::RS384,
    Algorithm::RS512,
    Algorithm::ES256,
    Algorithm::ES384,
    Algorithm::PS256,
    Algorithm::PS384,
    Algorithm::PS512,
    Algorithm::EdDSA,
];

pub fn check_bearer(
    jwk_set: &JwkSet,
    bearer_token: &str,
) -> Result<UserClaims, jsonwebtoken::errors::Error> {
    let unverified_header = jsonwebtoken::decode_header(bearer_token)?;

    let kid = unverified_header.kid.expect("Missing 'kid' claim");

    let jwk = jwk_set.find(&kid).expect("Invalid key id");

    let decoding_key = DecodingKey::from_jwk(jwk)?;

    let mut validation = Validation::new(VALID_ALGORITHMS[0]);
    validation.algorithms = VALID_ALGORITHMS.to_vec();
    validation.set_audience(&["plantswap"]);

    debug!("Trying to verify JWT");
    let verified_bearer = jsonwebtoken::decode(bearer_token, &decoding_key, &validation)?;

    debug!("Auth successful with claims {:?}", verified_bearer.claims);

    Ok(verified_bearer.claims)
}
