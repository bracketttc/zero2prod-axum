use crate::{authentication::UserId, error_handling::error_chain_fmt};
use anyhow::Context;
use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Extension,
};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
struct Dashboard<'a> {
    username: &'a str,
}

#[derive(thiserror::Error)]
pub enum DashError {
    #[error("User not logged in")]
    NotLoggedIn(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for DashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for DashError {
    fn into_response(self) -> axum::response::Response {
        match self {
            DashError::NotLoggedIn(_) => {
                tracing::warn!("{:?}", self);
                StatusCode::UNAUTHORIZED
            }
            DashError::UnexpectedError(_) => {
                tracing::error!("{:?}", self);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
        .into_response()
    }
}

pub async fn admin_dashboard(
    State(pool): State<Arc<PgPool>>,
    user_id: Extension<UserId>,
) -> Result<Response, DashError> {
    let username = get_username(**user_id, &pool)
        .await
        .map_err(DashError::UnexpectedError)?;

    let page = Dashboard {
        username: &username,
    }
    .render()
    .unwrap();
    Ok(Html(page).into_response())
}

#[tracing::instrument(name = "Get username", skip(pool))]
pub async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform query to retrieve a username.")?;
    Ok(row.username)
}
