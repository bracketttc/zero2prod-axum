use crate::helpers::spawn_app;
use fake::{
    faker::{internet::en::SafeEmail, name::en::Name},
    Fake,
};
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

fn valid_body() -> String {
    serde_urlencoded::to_string(&serde_json::json!({
        "name": Name().fake::<String>(),
        "email": SafeEmail().fake::<String>(),
    }))
    .unwrap()
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    let app = spawn_app().await;
    let body = valid_body();

    Mock::given(path("/api/1.0/messages/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let response = app.post_subscriptions(body).await;

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let app = spawn_app().await;
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = serde_urlencoded::to_string(serde_json::json!({
        "name": name,
        "email": email,
    }))
    .unwrap();

    Mock::given(path("/api/1.0/messages/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, email);
    assert_eq!(saved.name, name);
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let test_cases = vec![
        (
            serde_urlencoded::to_string(serde_json::json!({
                "name": Name().fake::<String>(),
            }))
            .unwrap(),
            "missing the email",
        ),
        (
            serde_urlencoded::to_string(serde_json::json!({
                "email": SafeEmail().fake::<String>(),
            }))
            .unwrap(),
            "missing the name",
        ),
        ("".to_string(), "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions(invalid_body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    let app = spawn_app().await;
    let test_cases = vec![
        (
            serde_urlencoded::to_string(serde_json::json!({
                "name": "",
                "email": SafeEmail().fake::<String>(),
            }))
            .unwrap(),
            "empty name",
        ),
        (
            serde_urlencoded::to_string(serde_json::json!({
                "name": Name().fake::<String>(),
                "email": ""
            }))
            .unwrap(),
            "empty email",
        ),
        (
            serde_urlencoded::to_string(serde_json::json!({
                "name": Name().fake::<String>(),
                "email": "definitely-not-an-email"
            }))
            .unwrap(),
            "invalid email",
        ),
    ];

    for (body, description) in test_cases {
        let response = app.post_subscriptions(body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 OK when the payload was {}.",
            description
        );
    }
}

#[tokio::test]
async fn subscribe_ends_a_confirmation_email_for_valid_data() {
    let app = spawn_app().await;
    let body = valid_body();

    Mock::given(path("/api/1.0/messages/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    // Mock asserts on drop
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let app = spawn_app().await;
    let body = valid_body();

    Mock::given(path("/api/1.0/messages/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    // Get first intercepted request
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    // the two links should be identical
    assert_eq!(confirmation_links.html, confirmation_links.text);
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    let app = spawn_app().await;
    let body = valid_body();

    // Sabotage the database
    sqlx::query!("ALTER TABLE subscriptions DROP COLUMN email;",)
        .execute(&app.db_pool)
        .await
        .unwrap();

    let response = app.post_subscriptions(body.into()).await;

    assert_eq!(response.status().as_u16(), 500);
}
