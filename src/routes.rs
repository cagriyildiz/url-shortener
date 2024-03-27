use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use base64::Engine;
use base64::engine::general_purpose;
use rand::Rng;
use sqlx::PgPool;
use url::Url;
use crate::utils::internal_error;
use crate::model::{Link, LinkTarget};

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "Service is healthy")
}

fn generate_id() -> String {
    let random_number = rand::thread_rng().gen_range(0..u32::MAX);
    general_purpose::URL_SAFE_NO_PAD.encode(random_number.to_string())
}

pub async fn create_link(
    State(pool): State<PgPool>,
    Json(new_link): Json<LinkTarget>,
) -> Result<Json<Link>, (StatusCode, String)> {
    let url = Url::parse(&new_link.target_url)
        .map_err(|_| (StatusCode::CONFLICT, "malformed url".into()))?
        .to_string();

    let new_link_id = generate_id();

    let insert_link_timeout = tokio::time::Duration::from_millis(300);

    let new_link = tokio::time::timeout(
        insert_link_timeout,
        sqlx::query_as!(
            Link,
            r#"
            with inserted_link as (
                insert into links(id, target_url)
                values($1, $2)
                returning id, target_url
            )
            select id, target_url from inserted_link
            "#,
            &new_link_id,
            &url
        )
        .fetch_one(&pool),
    )
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?;

    Ok(Json(new_link))
}