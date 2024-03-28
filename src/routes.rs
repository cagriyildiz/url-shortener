use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose;
use rand::Rng;
use sqlx::{Error, PgPool};
use sqlx::postgres::PgQueryResult;
use tokio::time::error::Elapsed;
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

    tracing::debug!("Created new link with id {} targeting {}", new_link_id, url);

    Ok(Json(new_link))
}

pub async fn redirect(
    State(pool): State<PgPool>,
    Path(requested_link): Path<String>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    let timeout = tokio::time::Duration::from_millis(300);

    let link = tokio::time::timeout(
        timeout,
        sqlx::query_as!(
            Link,
            "select id, target_url from links where id = $1",
            requested_link
        )
        .fetch_optional(&pool)
    )
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?
    .ok_or_else(|| "Link not found".to_string())
    .map_err(|err| (StatusCode::NOT_FOUND, err))?;

    tracing::debug!("Redirecting link id {} to {}", requested_link, link.target_url);

    let referer_header = headers
        .get("referer")
        .map(|val| val.to_str().unwrap_or_default().to_string());

    let user_agent_header = headers
        .get("user-agent")
        .map(|val| val.to_str().unwrap_or_default().to_string());

    let statistics = tokio::time::timeout(
        timeout,
        sqlx::query(
            "insert into link_statistics(link_id, referer, user_agent) values($1, $2, $3)"
        )
        .bind(&link.id)
        .bind(&referer_header)
        .bind(&user_agent_header)
        .execute(&pool)
    )
    .await;

    match statistics {
        Err(elapsed) => tracing::error!("Could not save new link click statistics due to a time out: {}", elapsed),
        Ok(Err(err)) => tracing::error!("Could not save new link click statistics due to an error: {}", err),
        _ => tracing::debug!(
            "Persisted new link click statistics for the link with id {}, referer {}, user_agent {}",
            requested_link,
            referer_header.unwrap_or_default(),
            user_agent_header.unwrap_or_default(),
        )
    }

    Ok(
        Response::builder()
            .status(StatusCode::TEMPORARY_REDIRECT)
            .header("Location", link.target_url)
            .body(Body::empty())
            .expect("Could not build response")
    )
}

pub async fn update_link(
    State(pool): State<PgPool>,
    Path(link_id): Path<String>,
    Json(new_link): Json<LinkTarget>,
) -> Result<Json<Link>, (StatusCode, String)> {
    let url = Url::parse(&new_link.target_url)
        .map_err(|_| (StatusCode::CONFLICT, "malformed url".into()))?
        .to_string();

    let update_link_timeout = tokio::time::Duration::from_millis(300);

    let updated_link = tokio::time::timeout(
        update_link_timeout,
        sqlx::query_as!(
            Link,
            r#"
            with update_link as (
                update links set target_url = $1 where id = $2
                returning id, target_url
            )
            select id, target_url from update_link
            "#,
            &url,
            &link_id
        )
        .fetch_one(&pool)
    )
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?;

    tracing::debug!("Updated link with id {}, now targeting {}", link_id, url);

    Ok(Json(updated_link))
}