use crate::authentication::UserId;
use askama::Template;
use axum::{
    response::{Html, IntoResponse, Response},
    Extension,
};
use axum_flash::IncomingFlashes;

#[derive(Template)]
#[template(path = "admin/password.html")]
struct PasswordChangeForm<'a> {
    flashes: &'a IncomingFlashes,
}

pub async fn change_password_form(
    _user_id: Extension<UserId>,
    flashes: IncomingFlashes,
) -> Response {
    let page = PasswordChangeForm { flashes: &flashes }.render().unwrap();
    (flashes, Html(page)).into_response()
}
