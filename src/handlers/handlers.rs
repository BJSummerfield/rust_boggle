use axum::{
    extract::{ws::WebSocketUpgrade, Form, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    Extension,
};

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::handlers::WebSockets;
use crate::models::{Boggle, PlayerId};
use crate::render::Render;

pub struct Handle {}

impl Handle {
    pub async fn root() -> impl IntoResponse {
        Html(Render::root()).into_response()
    }

    pub async fn new_game(Extension(boggle): Extension<Arc<Mutex<Boggle>>>) -> impl IntoResponse {
        let mut boggle = boggle.lock().await;

        boggle.new_game().await; // Reset the game state
        (StatusCode::NO_CONTENT, ())
    }

    pub async fn get_player_score(
        Extension(boggle): Extension<Arc<Mutex<Boggle>>>,
        Form(player_id): Form<PlayerId>,
    ) -> impl IntoResponse {
        let boggle = boggle.lock().await;
        let player_score_html = boggle.get_player_score(player_id).await;

        Html(player_score_html).into_response()
    }

    pub async fn websocket(
        ws: WebSocketUpgrade,
        State(state): State<Arc<Mutex<Boggle>>>,
    ) -> impl IntoResponse {
        ws.on_upgrade(|socket| async move {
            println!("Websocket connection received");
            WebSockets::new(socket, state).await
        })
    }
}
