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
    let pool = PgPool::connect_lazy(&connection_string.expose_secret())
        .expect("Failed to create Postgres connection pool.");
    let app = run(pool).unwrap().layer(TraceLayer::new_for_http());

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let socket: SocketAddr = address.parse().expect("Unable to parse socket address");
    Server::bind(socket)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
