use axum::{
    http::Request,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_sessions::SessionHandle;
use derive_more::Deref;
use uuid::Uuid;

#[derive(Copy, Clone, Debug, Deref)]
pub struct UserId(Uuid);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Middleware function that redirects requests to "/login" if they're not already logged in
pub async fn reject_anonymous_users<B>(mut request: Request<B>, next: Next<B>) -> Response {
    let session_handle = request.extensions().get::<SessionHandle>();
    if let Some(session_handle) = session_handle {
        let user_id = session_handle.read().await.get("user_id");
        if let Some(user_id) = user_id {
            request.extensions_mut().insert(UserId(user_id));
            return next.run(request).await;
        }
    }
    //request.uri().path_and_query();
    Redirect::to("/login").into_response()
}
