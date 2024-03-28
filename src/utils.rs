use axum::http::StatusCode;

pub fn internal_error<E>(err: E) -> (StatusCode, String) where
    E: std::error::Error,
{
    tracing::error!("{}", err);

    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}