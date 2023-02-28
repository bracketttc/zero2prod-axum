use crate::{
    authentication::reject_anonymous_users,
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{
        admin_dashboard, change_password, change_password_form, confirm, health_check, home,
        log_out, login, login_form, newsletter_form, publish_newsletter, subscribe, subscribe_form,
    },
};
use async_redis_session::RedisSessionStore;
use axum::{
    extract::FromRef,
    middleware::from_fn,
    routing::{get, post, IntoMakeService, Router},
};
use axum_flash::Key;
use axum_sessions::SessionLayer;
use axum_tracing_opentelemetry::opentelemetry_tracing_layer;
use hyper::{server::conn::AddrIncoming, Server};
use secrecy::{ExposeSecret, Secret};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{net::TcpListener, sync::Arc};

pub struct Application {
    port: u16,
    server: Server<AddrIncoming, IntoMakeService<Router>>,
}

#[derive(Clone, FromRef)]
pub struct AppState {
    pub connection_pool: Arc<PgPool>,
    pub email_client: Arc<EmailClient>,
    pub base_url: String,
    pub flash_config: axum_flash::Config,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
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
            configuration.application.hmac_secret,
            configuration.redis_uri,
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
    hmac_secret: Secret<String>,
    redis_uri: Secret<String>,
) -> Server<AddrIncoming, IntoMakeService<Router>> {
    let state = AppState {
        connection_pool: Arc::new(pool),
        email_client: Arc::new(email_client),
        base_url,
        flash_config: axum_flash::Config::new(Key::from(hmac_secret.expose_secret().as_bytes())),
    };

    // Redis store vs. local in-memory store
    let store = RedisSessionStore::new(redis_uri.expose_secret().to_string()).unwrap();
    //let store = MemoryStore::new();
    let session_layer = SessionLayer::new(store, hmac_secret.expose_secret().as_bytes());

    let app = Router::new()
        .route("/admin/dashboard", get(admin_dashboard))
        .route("/admin/logout", post(log_out))
        .route("/admin/password", get(change_password_form).post(change_password))
        .route("/admin/newsletter", get(newsletter_form).post(publish_newsletter))
        .layer(from_fn(reject_anonymous_users))
        .route("/", get(home))
        .route("/health_check", get(health_check))
        .route("/login", get(login_form).post(login))
        .route("/subscriptions", get(subscribe_form).post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .layer(session_layer)
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
