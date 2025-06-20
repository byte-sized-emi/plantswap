use std::str::FromStr as _;

use axum::{
    Extension, Router,
    extract::{Query, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Redirect},
    routing::get,
};
use axum_extra::extract::{cookie::{Cookie, SameSite}, CookieJar};
use axum_htmx::HxRequest;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, jwk::JwkSet};
use openidconnect::{
    AccessTokenHash, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet,
    EndpointNotSet, EndpointSet, HttpClientError, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, RequestTokenError,
    StandardErrorResponse, TokenResponse,
    core::{CoreAuthenticationFlow, CoreClient, CoreErrorResponseType, CoreProviderMetadata},
};
use serde::{Deserialize, Serialize};
use time::Duration;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{AppState, config::AppConfig};

pub mod layers;

const LOGIN_URL: &str = "/auth/login";
const REDIRECT_URL: &str = "/auth/redirect";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_route))
        .route("/redirect", get(redirect_route))
}

const BEARER_COOKIE_NAME: &str = "bearer";
const REFRESH_COOKIE_NAME: &str = "refresh";

const LOGIN_STATE_COOKIE: &str = "login_state";

/// This struct is serialized using json then sent to the user during
/// the login process. Because this is a cookie, it cannot be more than
/// 4KiB in size...it shouldn't, tho. unless the next_url is really really large.
#[derive(Deserialize, Serialize)]
struct LoginCookie {
    #[serde(rename = "csrf")]
    pub csrf_token: CsrfToken,
    #[serde(rename = "pkce")]
    pub pkce_verifier: PkceCodeVerifier,
    pub nonce: Nonce,
    #[serde(rename = "next")]
    #[serde(default)]
    pub next_url: Option<String>,
}

#[derive(Deserialize)]
struct NextUrlQuery {
    #[serde(default)]
    pub next: Option<String>,
}

async fn login_route(
    State(auth_state): State<AuthState>,
    mut jar: CookieJar,
    HxRequest(is_htmx): HxRequest,
    Query(next_url): Query<NextUrlQuery>,
) -> impl IntoResponse {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate the full authorization URL.
    let (auth_url, csrf_token, nonce) = auth_state
        .client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        // Set the desired scopes.
        // .add_scope(Scope::new("read".to_string()))
        // .add_scope(Scope::new("write".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let login_cookie = LoginCookie {
        csrf_token,
        pkce_verifier,
        nonce,
        next_url: next_url.next,
    };

    jar = jar.add(new_secure_cookie(
        LOGIN_STATE_COOKIE,
        serde_json::to_string(&login_cookie).unwrap(),
        Duration::minutes(10),
    ));

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
        .same_site(SameSite::Strict)
        .path("/")
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

    let login_cookie: LoginCookie = match jar.get(LOGIN_STATE_COOKIE) {
        Some(raw_cookie) => match serde_json::from_str(raw_cookie.value()) {
            Ok(cookie) => cookie,
            Err(err) => {
                warn!(?err, "Error while deserializing login_state cookie");
                jar = jar.remove(LOGIN_STATE_COOKIE);
                return (
                    StatusCode::BAD_REQUEST,
                    jar,
                    "Invalid login_state cookie".to_string(),
                )
                    .into_response();
            }
        },
        None => {
            error!("User tried accessing redirect route without login_state cookie");
            return (
                StatusCode::BAD_REQUEST,
                "Missing login_state cookie".to_string(),
            )
                .into_response();
        }
    };

    jar = jar.remove(LOGIN_STATE_COOKIE);

    let AuthzResp {
        state: new_state,
        code,
    } = query_params;

    if login_cookie.csrf_token != new_state {
        return (StatusCode::BAD_REQUEST, jar, "csrf state doesn't match").into_response();
    }

    let token_response = match auth_state
        .client
        .exchange_code(AuthorizationCode::new(code))
    {
        Ok(res) => res,
        Err(err) => {
            error!(?err, "error while trying to exchange code for bearer");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                jar,
                "Error while trying to exchange code for bearer".to_string(),
            )
                .into_response();
        }
    };

    let token_response = token_response
        .set_pkce_verifier(login_cookie.pkce_verifier)
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

    let access_token = token_response.access_token();
    let id_token = token_response.id_token()
        .expect("Server didn't supply id_token");
    let refresh_token = token_response.refresh_token();

    if refresh_token.is_none() {
        warn!("Refresh token is missing, activate refresh tokens for better security");
    }

    let id_token_verifier = auth_state.client.id_token_verifier();
    let maybe_claims = id_token
        .claims(&id_token_verifier, &login_cookie.nonce);

    let claims = match maybe_claims {
        Ok(claims) => claims,
        Err(err) => {
            warn!(?err, "Newly requested claims aren't valid");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Newly requested claims on id token aren't valid"
            ).into_response();
        }
    };

    if let Some(expected_access_token_hash) = claims.access_token_hash() {
        let actual_access_token_hash = AccessTokenHash::from_token(
            token_response.access_token(),
            id_token.signing_alg().unwrap(),
            id_token.signing_key(&id_token_verifier).unwrap(),
        )
        .unwrap();
        if actual_access_token_hash != *expected_access_token_hash {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    // FIXME: Make the duration's here either configurable,
    //        or extract them from the bearer/refresh JWT.
    //        This doesn't make a difference to security,
    //        as the token is checked on every request, but still.
    jar = jar.add(new_secure_cookie(
        BEARER_COOKIE_NAME,
        access_token.clone().into_secret(),
        Duration::days(100),
    ));

    if let Some(refresh_token) = refresh_token {
        jar = jar.add(new_secure_cookie(
            REFRESH_COOKIE_NAME,
            refresh_token.clone().into_secret(),
            Duration::days(365),
        ));
    }

    info!(
        user_id = claims.subject().as_str(),
        email = ?claims.email(),
        "Successfully logged in user"
    );

    (
        jar,
        Redirect::to(&login_cookie.next_url.unwrap_or("/".to_string())),
    )
        .into_response()
}

pub type OIDCClient = openidconnect::core::CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

#[derive(Clone)]
pub struct AuthState {
    pub jwk_set: JwkSet,
    pub client: OIDCClient,
    pub http_client: reqwest::Client,
}

pub async fn initialize_auth(config: &AppConfig) -> AuthState {
    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest Client should build");

    let provider_metadata = CoreProviderMetadata::discover_async(
        IssuerUrl::new(config.auth_server_url().to_string()).expect("Invalid issuer URL"),
        &http_client,
    )
        .await
        .expect("Couldn't discover OIDC metadata");

    // we can't use the jwk's that the openidconnect crate gives us,
    // so we convert them to the jsonwebtoken one
    let oidc_jwks = provider_metadata.jwks();
    let jwk_set: JwkSet = serde_json::from_str(&serde_json::to_string(oidc_jwks).unwrap()).unwrap();

    let redirect_url = RedirectUrl::new(format!("{}{REDIRECT_URL}", config.base_url())).unwrap();

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(config.auth_client_id().to_string()),
        Some(ClientSecret::new(config.auth_client_secret().to_string())),
    )
    .set_redirect_uri(redirect_url);

    AuthState {
        jwk_set,
        client,
        http_client,
    }
}

pub type Result<T> = std::result::Result<T, BackendError>;

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Error with HTTP request: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("OIDC error: {0}")]
    Oidc(
        #[from]
        RequestTokenError<
            HttpClientError<reqwest::Error>,
            StandardErrorResponse<CoreErrorResponseType>,
        >,
    ),

    #[error("OIDC Configuration error: {0}")]
    OidcConfiguration(
        #[from] openidconnect::ConfigurationError
    ),

    #[error("DB error: {0}")]
    Diesel(#[from] diesel::result::Error),

    #[error("Error during JWT validation: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Missing or invalid 'kid' claim on JWT")]
    MissingOrInvalidKidClaim,
}

impl IntoResponse for BackendError {
    fn into_response(self) -> askama_axum::Response {
        use BackendError::*;
        let status_code = match self {
            Jwt(_)
            | MissingOrInvalidKidClaim => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = if status_code == StatusCode::INTERNAL_SERVER_ERROR {
            "Internal server error, check logs for details".to_string()
        } else {
            format!("{self:?}")
        };

        (status_code, body).into_response()
    }
}

pub async fn base(
    State(auth_state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Result<impl IntoResponse> {
    let mut jar = CookieJar::from_headers(request.headers());
    let jwt = jar
        .get(BEARER_COOKIE_NAME)
        .map(|jwt| jwt.value().to_string());
    let refresh = jar
        .get(REFRESH_COOKIE_NAME)
        .map(|refresh| refresh.value().to_string());

    // Default context for unauthenticated requests
    let mut user: Option<UserClaims> = None;

    // JWT takes precedence if present
    if let Some(jwt) = jwt {
        match check_bearer(&auth_state.jwk_set, &jwt) {
            Ok(claims) => {
                user = Some(claims);
            }
            Err(err) => {
                warn!(?err, "Error while trying to check bearer");
                // Clear potentially compromised cookies
                jar = jar.remove(BEARER_COOKIE_NAME);
            }
        }
    }

    // Fall back to refresh token if JWT is absent/invalid
    if user.is_none() {
        if let Some(refresh) = refresh {
            let refresh_token_response = auth_state
                .client
                .exchange_refresh_token(&RefreshToken::new(refresh))?
                .request_async(&auth_state.http_client)
                .await;

            match refresh_token_response {
                Err(err) => {
                    warn!(?err, "Error while trying to exchange refresh token");
                    // Clear potentially compromised cookies
                    jar = jar.remove(REFRESH_COOKIE_NAME);
                }
                Ok(response) => {
                    let access_token = response.access_token();

                    jar = jar.add(new_secure_cookie(
                        BEARER_COOKIE_NAME,
                        access_token.secret().to_string(),
                        Duration::days(100),
                    ));

                    match check_bearer(&auth_state.jwk_set, access_token.secret()) {
                        Ok(claims) => {
                            user = Some(claims);
                        }
                        Err(jwt_err) => {
                            warn!(?jwt_err, "Error while checking validity of newly requested token. \
                                Removing bearer & refresh cookies");
                            // Clear potentially compromised cookies
                            jar = jar.remove(BEARER_COOKIE_NAME).remove(REFRESH_COOKIE_NAME);
                        }
                    }
                },
            };
        }
    }

    // Inject the resolved context into request extensions
    request.extensions_mut().insert(user);

    let response = next.run(request).await;

    // Merge cookie updates with the response
    Ok((jar, response).into_response())
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

pub fn check_bearer(
    jwk_set: &JwkSet,
    bearer_token: &str,
) -> Result<UserClaims> {
    let unverified_header = jsonwebtoken::decode_header(bearer_token)?;

    let kid = unverified_header.kid
        .ok_or(BackendError::MissingOrInvalidKidClaim)?;

    let jwk = jwk_set.find(&kid)
        .ok_or(BackendError::MissingOrInvalidKidClaim)?;

    let key_alg_name = jwk.common.key_algorithm.expect("JWK has no algorithm").to_string();
    let alg = Algorithm::from_str(&key_alg_name).unwrap();

    let decoding_key = DecodingKey::from_jwk(jwk)?;

    let mut validation = Validation::new(alg);
    validation.set_audience(&["plantswap"]);

    debug!("Trying to verify JWT");
    let verified_bearer = jsonwebtoken::decode(bearer_token, &decoding_key, &validation)?;

    debug!("Auth successful with claims {:?}", verified_bearer.claims);

    Ok(verified_bearer.claims)
}
