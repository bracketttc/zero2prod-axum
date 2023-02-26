use crate::{session_state::TypedSession, authentication::UserId};
use axum::{response::{IntoResponse, Redirect, Response}, Extension};
use axum_flash::Flash;

pub async fn log_out(
    flash: Flash,
    mut session: TypedSession,
    _user_id: Extension<UserId>,
) -> Response {
    session.log_out();
    (flash.info("You have successfully logged out."), Redirect::to("/login")).into_response()
}