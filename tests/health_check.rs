use axum::Server;
use std::net::TcpListener;
use zero2prod_axum::run;

fn spawn_app() {
    let app = run().expect("Failed to create router.");
    let listener = TcpListener::bind( "127.0.0.1:8000").expect("");
    tokio::spawn(async move {
        Server::from_tcp(listener).unwrap()
            .serve(app.into_make_service())
            .await
            .unwrap()
    });
}

#[tokio::test]
async fn health_check_works() {
    spawn_app();
    let client = reqwest::Client::new();

    let response = client
        .get("http://127.0.0.1:8000/health_check")
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
