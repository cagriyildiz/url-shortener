mod routes;
mod utils;
mod model;

use std::error::Error;
use axum::Router;
use axum::routing::{get, patch, post};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tracing::log::LevelFilter;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use crate::routes::{create_link, health, redirect, update_link};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or(
                    EnvFilter::new(tracing::Level::INFO.to_string())
                )
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_url = std::env::var("DATABASE_URL")
        .expect("Environment variable 'DATABASE_URL' is required");

    let db_pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&db_url)
        .await
        .expect("Could not connect to the database");

    let app = Router::new()
        .route("/create", post(create_link))
        .route("/:id", patch(update_link).get(redirect))
        .route("/health", get(health))
        .with_state(db_pool);

    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Could not initialize TcpListener");

    tracing::debug!(
        "listening on {}",
        listener
            .local_addr()
            .expect("Could not convert listener address to local address")
    );

    axum::serve(listener, app)
        .await
        .expect("Could not start the server");

    Ok(())
}
