use crate::routes::{health_check, subscribe};
use axum::routing::{get, post, Router};

pub fn run() -> Result<Router, std::io::Error> {
    let app = Router::new()
        //.route("/", get(|| greet(None)))
        //.route("/:name", get(greet))
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe));
    Ok(app)
}
