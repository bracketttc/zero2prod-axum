use std::time::Duration;

use crate::{configuration::Settings, startup::get_connection_pool};
use sqlx::PgPool;

#[tracing::instrument(skip_all)]
async fn expire_idempotency_keys(pool: &PgPool, ttl: u16) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        DELETE FROM idempotency
        WHERE
            EXTRACT(EPOCH FROM (now() - created_at)) > $1
        "#,
        ttl as i16,

    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn worker_loop(pool: PgPool, ttl: u16) -> Result<(), anyhow::Error> {
    loop {
        let result = expire_idempotency_keys(&pool, ttl).await;
        if result.is_err() {
            //todo!();
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

pub async fn run_worker_until_stopped(configuration: Settings) -> Result<(), anyhow::Error> {
    let pool = get_connection_pool(&configuration.database);
    let ttl = configuration.application.idempotency_ttl;
    worker_loop(pool, ttl).await
}