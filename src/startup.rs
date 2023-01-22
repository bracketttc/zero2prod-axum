use crate::routes::{health_check, subscribe};
use axum::routing::{get, post, Router};
use sqlx::PgPool;
use std::sync::Arc;

pub struct AppState {
    pub connection_pool: PgPool,
}

pub fn run(pool: PgPool) -> Result<Router, std::io::Error> {
    let app = Router::new()
        //.route("/", get(|| greet(None)))
        //.route("/:name", get(greet))
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(Arc::new(AppState {
            connection_pool: pool,
        }));
    Ok(app)
}
