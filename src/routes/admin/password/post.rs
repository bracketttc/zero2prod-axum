use crate::{
    authentication::{self, validate_credentials, AuthError, Credentials, UserId},
    domain::Password,
    error_handling::error_chain_fmt,
    routes::admin::dashboard::get_username,
};
use anyhow::Context;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Extension, Form,
};
use axum_flash::Flash;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(thiserror::Error)]
pub enum ChangePasswordError {
    #[error("The current password is incorrect.")]
    InvalidCredentials(#[from] anyhow::Error),
    #[error("Something unexpected happened")]
    UnexpectedError(#[source] anyhow::Error),
}

impl std::fmt::Debug for ChangePasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for ChangePasswordError {
    fn into_response(self) -> Response {
        match self {
            ChangePasswordError::InvalidCredentials(_) => {
                tracing::warn!("{self:?}");
            }
            ChangePasswordError::UnexpectedError(_) => {
                tracing::error!("{self:?}");
            }
        }

        ().into_response()
    }
}

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    pool: State<Arc<PgPool>>,
    flash: Flash,
    user_id: Extension<UserId>,
    Form(form): Form<FormData>,
) -> Result<Response, ChangePasswordError> {
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        let flash =
            flash.error("You entered two different new passwords - the field values must match.");
        return Ok((flash, Redirect::to("/admin/password")).into_response());
    }

    let new_password = match Password::parse(form.new_password) {
        Ok(password) => password,
        Err(e) => {
            return Ok(
                (flash.error(format!("{e}")), Redirect::to("/admin/password")).into_response(),
            );
        }
    };

    //let user_id = session.get_user_id().unwrap();
    let username = get_username(**user_id, &pool)
        .await
        .map_err(ChangePasswordError::UnexpectedError)?;
    // validate current password
    let credentials = Credentials {
        username,
        password: form.current_password,
    };
    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => Ok((
                flash.error("The current password is incorrect."),
                Redirect::to("/admin/password"),
            )
                .into_response()),
            AuthError::UnexpectedError(_) => Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        };
    }

    authentication::change_password(**user_id, new_password, &pool)
        .await
        .context("Failed to update user password")?;

    Ok((
        flash.info("Your password has been changed."),
        Redirect::to("/admin/password"),
    )
        .into_response())
}
