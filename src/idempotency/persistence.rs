use super::IdempotencyKey;
use axum::{
    headers::HeaderName,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use sqlx::{postgres::PgHasArrayType, PgPool, Postgres, Transaction};
use std::str::FromStr;
use uuid::Uuid;

pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(Response),
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

impl PgHasArrayType for HeaderPairRecord {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_header_pair")
    }
}

pub async fn get_saved_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<Option<Response>, anyhow::Error> {
    let saved_response = sqlx::query!(
        r#"
        SELECT
            response_status_code as "response_status_code!",
            response_headers as "response_headers!: Vec<HeaderPairRecord>",
            response_body as "response_body!"
        FROM idempotency
        WHERE
            user_id = $1 AND
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .fetch_optional(pool)
    .await?;

    if let Some(r) = saved_response {
        let status_code = StatusCode::from_u16(r.response_status_code.try_into()?)?;
        let headers = {
            let mut headers = HeaderMap::new();
            for HeaderPairRecord { name, value } in r.response_headers {
                headers.append(
                    HeaderName::from_str(name.as_str())?,
                    HeaderValue::try_from(value)?,
                );
            }
            headers
        };
        let response = (status_code, headers, r.response_body).into_response();
        Ok(Some(response))
    } else {
        Ok(None)
    }
}

pub async fn save_response(
    mut transaction: Transaction<'static, Postgres>,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
    response: Response,
) -> Result<Response, anyhow::Error> {
    let (parts, body) = response.into_parts();
    let status_code = parts.status.as_u16() as i16;
    let body = hyper::body::to_bytes(body).await?;

    let headers = {
        let mut h = Vec::with_capacity(parts.headers.len());
        for (name, value) in parts.headers.iter() {
            let name = name.as_str().to_owned();
            let value = value.as_bytes().to_owned();
            h.push(HeaderPairRecord { name, value });
        }
        h
    };

    // Can't check use of custom types at compile-time
    sqlx::query_unchecked!(
        r#"
        UPDATE idempotency
        SET
            response_status_code = $3,
            response_headers = $4,
            response_body = $5
        WHERE
            user_id = $1 AND
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body.as_ref(),
    )
    .execute(&mut transaction)
    .await?;
    transaction.commit().await?;

    let response = (parts, body).into_response();
    Ok(response)
}

pub async fn try_processing(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<NextAction, anyhow::Error> {
    let mut transaction = pool.begin().await?;
    let n_inserted_rows = sqlx::query!(
        r#"
        INSERT INTO idempotency (
            user_id,
            idempotency_key,
            created_at
        )
        VALUES ($1, $2, now())
        ON CONFLICT DO NOTHING
        "#,
        user_id,
        idempotency_key.as_ref(),
    )
    .execute(&mut transaction)
    .await?
    .rows_affected();

    if n_inserted_rows > 0 {
        Ok(NextAction::StartProcessing(transaction))
    } else {
        let saved_response = get_saved_response(pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("We expected a saved response, we didn't find it"))?;
        Ok(NextAction::ReturnSavedResponse(saved_response))
    }
}
