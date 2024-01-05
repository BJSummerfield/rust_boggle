use axum::{
    routing::{get, post},
    Extension, Router,
};
use std::{env, net::SocketAddr, sync::Arc};
use tower_http::services::ServeDir;

mod boggle;
mod dictionary;
mod gamestate;
mod handlers;
mod player_state;
mod render;

use gamestate::GameState;
use handlers::Handle;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let game_state = GameState::new();
    let styles_path = env::var("STATIC_FILES_PATH").unwrap_or_else(|_| "/app/static".to_string());
    let app = Router::new()
        .route("/", get(Handle::root))
        .route("/new_game", post(Handle::new_game))
        .route("/get_score", post(Handle::get_player_score))
        .layer(Extension(Arc::clone(&game_state)))
        .nest_service("/static", ServeDir::new(styles_path))
        .route("/ws", get(Handle::websocket))
        .with_state(game_state);

    // Bind to a socket address
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Listening on {}", addr);

    // Run the server
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
