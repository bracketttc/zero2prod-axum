use axum_server::Server;
use std::net::SocketAddr;
use zero2prod_axum::configuration::get_configuration;
use zero2prod_axum::startup::run;

#[tokio::main]
async fn main() {
    let configuration = get_configuration().expect("Failed to read configuration.");
    let app = run().unwrap();
    Server::bind(SocketAddr::from((
        [127, 0, 0, 1],
        configuration.application_port,
    )))
    .serve(app.into_make_service())
    .await
    .unwrap();
}
