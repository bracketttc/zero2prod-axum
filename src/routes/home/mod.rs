use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};

pub async fn home() -> impl IntoResponse {
    (StatusCode::OK, Html(include_str!("home.html")))
}
