use std::sync::Arc;

use axum::{async_trait, extract::{FromRequestParts, Query, State}, http::{header::AUTHORIZATION, request::Parts, StatusCode}, response::{IntoResponse, Redirect}, routing::get, Json, Router};
use jsonwebtoken::{jwk::JwkSet, DecodingKey, Validation};
use oauth2::{basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, RedirectUrl, TokenResponse, TokenUrl};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{config::AppConfig, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_route))
        .route("/redirect", get(redirect_route))
}

/// TODO: Store CSRF token and use PKCE challenge
async fn login_route(State(auth_state): State<Arc<AuthState>>, user: Option<UserClaims>) -> impl IntoResponse {
    if user.is_some() {
        Redirect::to("/")
    } else {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // Generate the full authorization URL.
        let (auth_url, csrf_token) = auth_state.oauth2_client
            .authorize_url(CsrfToken::new_random)
            // Set the desired scopes.
            // .add_scope(Scope::new("read".to_string()))
            // .add_scope(Scope::new("write".to_string()))
            // Set the PKCE code challenge.
            // .set_pkce_challenge(pkce_challenge)
            .url();

        Redirect::to(auth_url.as_str())
    }
}

#[derive(Debug, Deserialize)]
struct RedirectRouteQueryParams {
    pub state: String,
    pub code: String,
}

#[derive(Serialize)]
struct AccessAndRefreshToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

/// The user gets redirected back here from the auth provider after a successful login.
/// TODO: Verify that csrf_token matches the csrf_token from the login route
/// and verify the pkce challenge
async fn redirect_route(
    State(auth_state): State<Arc<AuthState>>,
    Query(query_params): Query<RedirectRouteQueryParams>
) -> Json<AccessAndRefreshToken> {
    // TODO: Verify CSRF token
    let token_result = auth_state.oauth2_client.exchange_code(AuthorizationCode::new(query_params.code))
        // .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await.unwrap();

    Json(AccessAndRefreshToken {
        access_token: token_result.access_token().secret().to_string(),
        refresh_token: token_result.refresh_token().map(|t| t.secret().to_string())
    })
}

#[derive(Debug)]
pub struct AuthState {
    pub jwk_set: JwkSet,
    pub oauth2_client: BasicClient,
}

pub async fn initialize_auth(config: &AppConfig) -> AuthState {
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

    AuthState { jwk_set, oauth2_client }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserClaims {
    #[serde(rename = "sub")]
    pub user_id: String,
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub realm_roles: Vec<String>,
}

#[async_trait]
impl FromRequestParts<AppState> for UserClaims
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let header = parts.headers.get(AUTHORIZATION)
            .ok_or((StatusCode::BAD_REQUEST, "Missing auth header".to_string()))?
            .to_str()
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid auth header".to_string()))?;

        let header = header.strip_prefix("Bearer: ")
            .ok_or((StatusCode::BAD_REQUEST, "No bearer in Auth header".to_string()))?;

        check_bearer(&state.auth_state.jwk_set, header).await
    }
}

pub async fn check_bearer(jwk_set: &JwkSet, bearer_token: &str) -> Result<UserClaims, (StatusCode, String)> {
    let unverified_header =
        jsonwebtoken::decode_header(bearer_token).map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;

    let kid = unverified_header.kid.ok_or((StatusCode::BAD_REQUEST, "Missing 'kid' claim".to_string()))?;

    let jwk = jwk_set
        .find(&kid)
        .ok_or((StatusCode::BAD_REQUEST,format!("Could not find key id {}", kid)))?;

    let decoding_key = DecodingKey::from_jwk(jwk).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "".to_string()))?;

    let mut validation = Validation::new(unverified_header.alg);
    validation.set_audience(&["plantswap"]);

    debug!("Trying to verify JWT");
    let verified_header = jsonwebtoken::decode(bearer_token, &decoding_key, &validation)
        .map_err(|err| (StatusCode::UNAUTHORIZED, format!("Token verification failed: {err:?}")))?;

    debug!("Auth successful with claims {:?}", verified_header.claims);

    Ok(verified_header.claims)
}
