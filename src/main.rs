use axum_server::Server;
use secrecy::ExposeSecret;
use sqlx::PgPool;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use zero2prod_axum::configuration::get_configuration;
use zero2prod_axum::startup::run;
use zero2prod_axum::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() {
    let subscriber = get_subscriber("zero2prod_axum".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_string = configuration.database.connection_string();
    let pool = PgPool::connect(&connection_string.expose_secret())
        .await
        .expect("Failed to connect to Postgres.");
    let app = run(pool).unwrap().layer(TraceLayer::new_for_http());
    Server::bind(SocketAddr::from((
        [127, 0, 0, 1],
        configuration.application_port,
    )))
    .serve(app.into_make_service())
    .await
    .unwrap();
}
