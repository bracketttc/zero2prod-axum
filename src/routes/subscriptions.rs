use axum::{extract::Form, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(form: Form<FormData>) -> impl IntoResponse {
    StatusCode::OK
}
