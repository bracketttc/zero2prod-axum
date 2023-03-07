use std::ops::{Deref, DerefMut};

use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use axum_sessions::extractors::WritableSession;
use uuid::Uuid;

/// A type-safe wrapper for the `axum_sessions::extractors::WritableSession`.
pub struct TypedSession(WritableSession);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";

    /// Generates a new id and cookie for this session.
    pub fn renew(&mut self) {
        self.0.regenerate();
    }

    pub fn log_out(&mut self) {
        self.0.destroy()
    }
    pub fn insert_user_id(&mut self, user_id: Uuid) -> Result<(), serde_json::Error> {
        self.0.insert(Self::USER_ID_KEY, user_id)
    }

    pub fn get_user_id(&self) -> Option<Uuid> {
        self.0.get(Self::USER_ID_KEY)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for TypedSession
where
    S: Send + Sync,
{
    type Rejection = <WritableSession as FromRequestParts<S>>::Rejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(
            WritableSession::from_request_parts(parts, state).await?,
        ))
    }
}

impl Deref for TypedSession {
    type Target = WritableSession;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TypedSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
