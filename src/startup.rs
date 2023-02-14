use crate::{
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{confirm, health_check, publish_newsletter, subscribe},
};
use axum::routing::{get, post, IntoMakeService, Router};
use axum_tracing_opentelemetry::opentelemetry_tracing_layer;
use hyper::{server::conn::AddrIncoming, Server};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{net::TcpListener, sync::Arc};

pub struct Application {
    port: u16,
    server: Server<AddrIncoming, IntoMakeService<Router>>,
}

pub struct AppState {
    pub connection_pool: PgPool,
    pub email_client: EmailClient,
    pub base_url: String,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(&configuration.database);

        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address.");
        let timeout = configuration.email_client.timeout();
        let email_client = EmailClient::new(
            configuration.email_client.base_url.clone(),
            sender_email,
            configuration.email_client.api_key.clone(),
            timeout,
        );

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        //let socket: SocketAddr = address.parse().expect("Unable to parse socket address");
        let listener = TcpListener::bind(address).expect("Failed to bind port.");
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            connection_pool,
            email_client,
            configuration.application.base_url,
        );

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), hyper::Error> {
        self.server.await
    }
}

pub fn run(
    listener: TcpListener,
    pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Server<AddrIncoming, IntoMakeService<Router>> {
    let state = AppState {
        connection_pool: pool,
        email_client,
        base_url,
    };
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/newsletters", post(publish_newsletter))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .layer(opentelemetry_tracing_layer())
        .with_state(Arc::new(state));
    Server::from_tcp(listener)
        .expect("Failed to connect to socket")
        .serve(app.into_make_service())
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}
