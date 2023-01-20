use axum::Server;
use std::net::TcpListener;
use zero2prod_axum::run;

fn spawn_app() -> String {
    let app = run().expect("Failed to create router.");
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind port.");
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        Server::from_tcp(listener)
            .unwrap()
            .serve(app.into_make_service())
            .await
            .unwrap()
    });
    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn health_check_works() {
    let host = spawn_app();
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health_check", host))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
