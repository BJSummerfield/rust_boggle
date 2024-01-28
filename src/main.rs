use axum::{
    error_handling::HandleErrorLayer,
    http::StatusCode,
    routing::{get, post},
    BoxError, Extension, Router,
};
use std::{env, net::SocketAddr, sync::Arc};
use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};
use tower_http::services::ServeDir;
use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};

mod handlers;
mod models;
mod render;

use handlers::Handle;
use models::Boggle;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_expiry(Expiry::OnSessionEnd);

    let boggle = Boggle::new();
    let styles_path = env::var("STATIC_FILES_PATH").unwrap_or_else(|_| "/app/static".to_string());
    let app = Router::new()
        .route("/", get(Handle::root))
        .route("/username", post(Handle::username))
        .route("/submit_word", post(Handle::submit_word))
        .route("/new_game", post(Handle::new_game))
        .route("/get_score", post(Handle::get_player_score))
        .layer(Extension(Arc::clone(&boggle)))
        .nest_service("/static", ServeDir::new(styles_path))
        .route("/ws", get(Handle::websocket))
        .with_state(boggle)
        .layer(session_layer)
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|err: BoxError| async move {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled error: {}", err),
                    )
                }))
                .layer(BufferLayer::new(1024))
                .layer(RateLimitLayer::new(5, std::time::Duration::from_secs(1))) // Rate limit: 5 requests per second
                .into_inner(),
        );

    // Bind to a socket address
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    // Run the server
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
