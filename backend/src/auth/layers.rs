use axum::{
    body::Body,
    http::{Request as HttpRequest, Response},
    response::{IntoResponse, Redirect},
};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};

use super::{LOGIN_URL, UserClaims};

/// Convenience function that calls `RequireLogin::new()`.
/// This creates a layer which requires users to be logged in,
/// redirecting them to the login page if they are not authenticated.
pub const fn require_login() -> RequireLogin {
    RequireLogin::new()
}

/// A middleware layer that requires users to be logged in to access the route.
///
/// Usage:
/// ```rust
/// app.route_layer(RequireLogin::new())
/// ```
#[derive(Clone)]
pub struct RequireLogin;

impl RequireLogin {
    pub const fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for RequireLogin {
    type Service = RequireLoginMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequireLoginMiddleware { inner }
    }
}

#[derive(Clone)]
pub struct RequireLoginMiddleware<S> {
    inner: S,
}

impl<S> Service<HttpRequest<Body>> for RequireLoginMiddleware<S>
where
    S: Service<HttpRequest<Body>, Response = Response<Body>> + Send + Sync + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: HttpRequest<Body>) -> Self::Future {
        let has_user = req
            .extensions()
            .get::<Option<UserClaims>>()
            .cloned()
            .flatten();

        if has_user.is_none() {
            let response = Redirect::to(LOGIN_URL).into_response();
            return Box::pin(async move { Ok(response) });
        }

        req.extensions_mut().insert(has_user.unwrap());

        let future = self.inner.call(req);
        Box::pin(async move {
            let response: Response<Body> = future.await?;
            Ok(response)
        })
    }
}
