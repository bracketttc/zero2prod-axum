use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    error_handling::error_chain_fmt,
};
use axum::{
    extract::{Form, State},
    response::{IntoResponse, Redirect},
};
use axum_flash::Flash;
use secrecy::Secret;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for LoginError {
    fn into_response(self) -> axum::response::Response {
        match self {
            LoginError::AuthError(_) => tracing::warn!("{:?}", &self),
            LoginError::UnexpectedError(_) => tracing::error!("{:?}", &self),
        };

        let query_string = format!("error={}", urlencoding::Encoded::new(self.to_string()));
        Redirect::to(format!("/login?{query_string}").as_str()).into_response()
    }
}

#[tracing::instrument(
    skip(pool, flash, form),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(pool): State<Arc<PgPool>>,
    flash: Flash,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            Redirect::to("/").into_response()
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };

            (
                flash.error(format!("{e}")),
                Redirect::to("/login")
            )
            .into_response()
        }
    }
}
