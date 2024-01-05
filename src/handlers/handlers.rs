use axum::{
    extract::{ws::WebSocketUpgrade, Form, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    Extension,
};

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::handlers::WebSockets;
use crate::render::Render;
use crate::GameState;

#[derive(serde::Deserialize)]
pub struct PlayerName {
    pub username: String,
}

pub struct Handle {}

impl Handle {
    pub async fn root() -> impl IntoResponse {
        Html(Render::root()).into_response()
    }

    pub async fn new_game(
        Extension(gamestate): Extension<Arc<Mutex<GameState>>>,
    ) -> impl IntoResponse {
        let mut gamestate = gamestate.lock().await;

        gamestate.new_game().await; // Reset the game state
        (StatusCode::NO_CONTENT, ())
    }

    pub async fn get_player_score(
        Extension(gamestate): Extension<Arc<Mutex<GameState>>>,
        Form(PlayerName { username }): Form<PlayerName>,
    ) -> impl IntoResponse {
        let gamestate = gamestate.lock().await;
        let player_score_html = gamestate.get_player_score(&username).await;

        Html(player_score_html).into_response()
    }

    pub async fn websocket(
        ws: WebSocketUpgrade,
        State(state): State<Arc<Mutex<GameState>>>,
    ) -> impl IntoResponse {
        ws.on_upgrade(|socket| async move {
            println!("Websocket connection received");
            WebSockets::new(socket, state).await
        })
    }
}
