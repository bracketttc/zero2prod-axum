use axum_server::Server;
use std::net::SocketAddr;
use zero2prod_axum::run;

#[tokio::main]
async fn main() {
    let app = run().unwrap();
    Server::bind(SocketAddr::from(([127, 0, 0, 1], 8000)))
        .serve(app.into_make_service())
        .await
        .unwrap();
}
