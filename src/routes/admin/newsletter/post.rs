use crate::{
    authentication::UserId,
    error_handling::error_chain_fmt,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
};
use anyhow::Context;
use axum::{
    extract::{Form, State},
    http::{
        header::{HeaderMap, HeaderValue},
        StatusCode,
    },
    response::{IntoResponse, Redirect, Response},
    Extension,
};

use axum_flash::Flash;
use hyper::header;
use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    html: String,
    text: String,
    idempotency_key: String,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed.")]
    Authentication(#[source] anyhow::Error),
    #[error("{0}")]
    BadRequest(#[source] anyhow::Error),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
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
            PublishError::Authentication(_) => {
                tracing::warn!("{self:?}");
                let mut headers = HeaderMap::new();
                headers.insert(
                    header::WWW_AUTHENTICATE,
                    HeaderValue::from_str(r#"Basic realm="publish""#).unwrap(),
                );
                (StatusCode::UNAUTHORIZED, headers, body).into_response()
            }
            PublishError::BadRequest(_) => {
                tracing::warn!("{self:?}");
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            PublishError::Unexpected(_) => {
                tracing::error!("{self:?}");
                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            }
        }
    }
}

#[tracing::instrument(name = "Publish a newsletter", skip_all, fields(user_id=%&*user_id))]
pub async fn publish_newsletter(
    State(pool): State<Arc<PgPool>>,
    flash: Flash,
    user_id: Extension<UserId>,
    Form(body): Form<BodyData>,
) -> Result<Response, PublishError> {
    let idempotency_key: IdempotencyKey = body
        .idempotency_key
        .try_into()
        .map_err(PublishError::BadRequest)?;
    let mut transaction = match try_processing(&pool, &idempotency_key, **user_id).await? {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            return Ok(saved_response);
        }
    };

    let issue_id = insert_newsletter_issue(&mut transaction, &body.title, &body.text, &body.html)
        .await
        .context("Failed to store newsletter issue details")
        .map_err(PublishError::Unexpected)?;

    enqueue_delivery_tasks(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue email delivery tasks")
        .map_err(PublishError::Unexpected)?;

    let response = (
        flash.info("The newsletter issue has been accepted - emails will go out shortly."),
        Redirect::to("/admin/newsletter"),
    )
        .into_response();
    let response = save_response(transaction, &idempotency_key, **user_id, response)
        .await
        .map_err(PublishError::Unexpected)?;

    Ok(response)
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            text_content,
            html_content,
            published_at
        )
        VALUES ($1, $2, $3, $4, now())
        "#,
        newsletter_issue_id,
        title,
        text_content,
        html_content,
    )
    .execute(transaction)
    .await?;
    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        SELECT $1, email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
        newsletter_issue_id,
    )
    .execute(transaction)
    .await?;
    Ok(())
}
