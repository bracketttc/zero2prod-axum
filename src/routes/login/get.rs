use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};
use axum_flash::IncomingFlashes;
//use std::fmt::Write;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginForm<'a> {
    flashes: &'a IncomingFlashes,
}

pub async fn login_form(flashes: IncomingFlashes) -> impl IntoResponse {
    let page = LoginForm { flashes: &flashes }.render().unwrap();
    (StatusCode::OK, flashes, Html(page))
}
