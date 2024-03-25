mod routes;

use std::error::Error;
use axum::Router;
use axum::routing::get;
use tokio::net::TcpListener;
use crate::routes::health;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let app = Router::new()
        .route("/health", get(health));

    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Could not initialize TcpListener");

    axum::serve(listener, app)
        .await
        .expect("Could not start the server");

    Ok(())
}
