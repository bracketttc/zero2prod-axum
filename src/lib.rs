use axum::{extract::Path, http::StatusCode, response::IntoResponse, routing::get, Router};

async fn greet(name: Option<Path<String>>) -> impl IntoResponse {
    match name {
        Some(Path(name)) => format!("Hello {}!", name),
        None => "Hello World!".into(),
    }
}

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

pub fn run() -> Result<Router, std::io::Error> {
    let app = Router::new()
        .route("/", get(|| greet(None)))
        .route("/:name", get(greet))
        .route("/health_check", get(health_check));
    Ok(app)
}
