use crate::{
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{confirm, health_check, home, login, login_form, publish_newsletter, subscribe},
};
use axum::{
    extract::FromRef,
    routing::{get, post, IntoMakeService, Router},
};
use axum_flash::Key;
use axum_tracing_opentelemetry::opentelemetry_tracing_layer;
use hyper::{server::conn::AddrIncoming, Server};
use secrecy::{ExposeSecret, Secret};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{net::TcpListener, ops::Deref, sync::Arc};

pub struct Application {
    port: u16,
    server: Server<AddrIncoming, IntoMakeService<Router>>,
}

#[derive(Clone)]
pub struct HmacSecret(pub Secret<String>);

impl Deref for HmacSecret {
    type Target = Secret<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, FromRef)]
pub struct AppState {
    pub connection_pool: Arc<PgPool>,
    pub email_client: Arc<EmailClient>,
    pub base_url: String,
    pub flash_config: axum_flash::Config,
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

        let listener = TcpListener::bind(address).expect("Failed to bind port.");
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            connection_pool,
            email_client,
            configuration.application.base_url,
            axum_flash::Config::new(Key::from(
                configuration
                    .application
                    .hmac_secret
                    .expose_secret()
                    .as_bytes(),
            )),
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
    flash_config: axum_flash::Config,
) -> Server<AddrIncoming, IntoMakeService<Router>> {
    let state = AppState {
        connection_pool: Arc::new(pool),
        email_client: Arc::new(email_client),
        base_url,
        flash_config,
    };
    let app = Router::new()
        .route("/", get(home))
        .route("/health_check", get(health_check))
        .route("/login", get(login_form))
        .route("/login", post(login))
        .route("/newsletters", post(publish_newsletter))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .layer(opentelemetry_tracing_layer())
        .with_state(state);
    Server::from_tcp(listener)
        .expect("Failed to connect to socket")
        .serve(app.into_make_service())
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}
