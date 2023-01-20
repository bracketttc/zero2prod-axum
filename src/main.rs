use axum::{extract::Path, response::IntoResponse, routing::get, Router};
use std::net::SocketAddr;

async fn greet(name: Option<Path<String>>) -> impl IntoResponse {
    match name {
        Some(Path(name)) => format!("Hello {}!", name),
        None => "Hello World!".into(),
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(|| greet(None)))
        .route("/:name", get(greet));
    axum_server::bind(SocketAddr::from(([127, 0, 0, 1], 8000)))
        .serve(app.into_make_service())
        .await
        .unwrap();
}
