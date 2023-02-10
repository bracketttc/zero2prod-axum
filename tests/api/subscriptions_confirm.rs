use crate::helpers::spawn_app;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

fn valid_request_body() -> &'static str {
    "name=le%20guin&email=ursula_le_guin%40gmail.com"
}

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!("{0}/subscriptions/confirm", app.host))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    let app = spawn_app().await;

    Mock::given(path("/api/1.0/messages/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(valid_request_body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    let app = spawn_app().await;

    Mock::given(path("/api/1.0/messages/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(valid_request_body().into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");
}

#[tokio::test]
async fn confirmation_link_returns_500_on_database_error() {
    let app = spawn_app().await;

    Mock::given(path("/api/1.0/messages/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(valid_request_body().into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    // Sabotage the database
    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token;",)
        .execute(&app.db_pool)
        .await
        .unwrap();

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 500);
}

#[tokio::test]
async fn confirmations_with_invalid_tokens_are_rejected_with_401() {
    let app = spawn_app().await;

    // 'foo' isn't a valid token, and the database contains
    let response = reqwest::get(&format!(
        "{}/subscriptions/confirm?subscription_token=foo",
        &app.host
    ))
    .await
    .unwrap();

    assert_eq!(response.status().as_u16(), 401);
}
