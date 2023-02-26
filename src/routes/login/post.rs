use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    error_handling::error_chain_fmt,
    session_state::TypedSession,
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

        Redirect::to("/login").into_response()
    }
}

pub struct FlashError {
    flash: Flash,
    e: LoginError,
}

impl IntoResponse for FlashError {
    fn into_response(self) -> axum::response::Response {
        (self.flash.error(self.e.to_string()), self.e).into_response()
    }
}

#[tracing::instrument(
    skip(pool, flash, session, form),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(pool): State<Arc<PgPool>>,
    flash: Flash,
    mut session: TypedSession,
    Form(form): Form<FormData>,
) -> Result<impl IntoResponse, FlashError> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            session.regenerate();
            session.insert_user_id(user_id).map_err(|e| FlashError {
                flash,
                e: LoginError::UnexpectedError(e.into()),
            })?;
            Ok((Redirect::to("/admin/dashboard")).into_response())
        }
        Err(e) => Err(FlashError {
            flash,
            e: match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            },
        }),
    }
}
