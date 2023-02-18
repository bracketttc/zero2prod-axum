use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    domain::SubscriberEmail,
    email_client::EmailClient,
    error_handling::error_chain_fmt,
};
use anyhow::Context;
use axum::{
    extract::{Json, State},
    http::{
        header::{HeaderMap, HeaderValue},
        StatusCode,
    },
    response::IntoResponse,
};
use base64::Engine;
use hyper::header;
use secrecy::Secret;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for PublishError {
    fn into_response(self) -> axum::response::Response {
        let body = format!("{self}");
        match self {
            PublishError::AuthError(_) => {
                tracing::warn!("{:?}", &self);
                let mut headers = HeaderMap::new();
                headers.insert(
                    header::WWW_AUTHENTICATE,
                    HeaderValue::from_str(r#"Basic realm="publish""#).unwrap(),
                );
                (StatusCode::UNAUTHORIZED, headers, body).into_response()
            }
            PublishError::UnexpectedError(_) => {
                tracing::error!("{:?}", &self);
                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            }
        }
    }
}

#[tracing::instrument(
    name = "Publish a newsletter",
    skip(pool, email_client, headers, body),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    State(pool): State<Arc<PgPool>>,
    State(email_client): State<Arc<EmailClient>>,
    headers: HeaderMap,
    Json(body): Json<BodyData>,
) -> Result<impl IntoResponse, PublishError> {
    let credentials = basic_authentication(headers).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => PublishError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => PublishError::UnexpectedError(e.into()),
        })?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&pool)
        .await
        .context("Failed to get confirmed subscribers")?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!( error.cause_chain = ?error, "Skipping a confirmed subscriber. \
            Their stored contact details are invalid",);
            }
        }
    }

    Ok(StatusCode::OK)
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?;

    let confirmed_subscribers = rows
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();
    Ok(confirmed_subscribers)
}

fn basic_authentication(headers: HeaderMap) -> Result<Credentials, anyhow::Error> {
    // Using axum::extract::TypedHeader to pull out a Authorization<Basic> is much less verbose, but
    // when converted into a response yields 400 BAD REQUEST and not 401 UNAUTHORIZED and seems not to
    // allow for the response to include a realm
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("Failed to base64-decode 'Basic' credentials.")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    let password = Secret::new(
        credentials
            .next()
            .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
            .to_string(),
    );

    Ok(Credentials { username, password })
}