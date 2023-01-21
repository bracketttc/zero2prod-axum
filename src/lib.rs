use axum::{
    extract::{Form, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct FormData {
    email: String,
    name: String,
}

async fn greet(name: Option<Path<String>>) -> impl IntoResponse {
    match name {
        Some(Path(name)) => format!("Hello {}!", name),
        None => "Hello World!".into(),
    }
}

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

async fn subscribe(form: Form<FormData>) -> impl IntoResponse {
    StatusCode::OK
}

pub fn run() -> Result<Router, std::io::Error> {
    let app = Router::new()
        .route("/", get(|| greet(None)))
        .route("/:name", get(greet))
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe));
    Ok(app)
}
