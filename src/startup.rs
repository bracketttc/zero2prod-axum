use crate::{
    configuration::Settings,
    email_client::EmailClient,
    routes::{health_check, subscribe},
};
use axum::routing::{get, post, Router};
use axum_tracing_opentelemetry::opentelemetry_tracing_layer;
use hyper::Server;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{future::Future, net::TcpListener, sync::Arc};

pub fn build(configuration: Settings) -> impl Future<Output = hyper::Result<()>> {
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());

    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address.");
    let timeout = configuration.email_client.timeout();
    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.api_key,
        timeout,
    );

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    //let socket: SocketAddr = address.parse().expect("Unable to parse socket address");
    let listener = TcpListener::bind(address).expect("Failed to bind port.");
    run(listener, connection_pool, email_client)
}

pub fn run(
    listener: TcpListener,
    pool: PgPool,
    email_client: EmailClient,
) -> impl Future<Output = hyper::Result<()>> {
    let app = Router::new()
        //.route("/", get(|| greet(None)))
        //.route("/:name", get(greet))
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .layer(opentelemetry_tracing_layer())
        .with_state(Arc::new(pool))
        .with_state(Arc::new(email_client));
    Server::from_tcp(listener)
        .expect("Failed to connect to socket")
        .serve(app.into_make_service())
}
