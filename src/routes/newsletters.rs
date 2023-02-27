use crate::{
    authentication::UserId, domain::SubscriberEmail, email_client::EmailClient,
    error_handling::error_chain_fmt,
};
use anyhow::Context;
use axum::{
    extract::{Form, State},
    http::{
        header::{HeaderMap, HeaderValue},
        StatusCode,
    },
    response::IntoResponse,
    Extension,
};

use hyper::header;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
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

#[tracing::instrument(name = "Publish a newsletter", skip(pool, email_client, body))]
pub async fn publish_newsletter(
    State(pool): State<Arc<PgPool>>,
    State(email_client): State<Arc<EmailClient>>,
    user_id: Extension<UserId>,
    Form(body): Form<BodyData>,
) -> Result<impl IntoResponse, PublishError> {
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
                        &body.html,
                        &body.text,
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
