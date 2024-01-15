use axum::{
    extract::{ws::WebSocketUpgrade, Form, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    Extension,
};

use std::sync::Arc;
use tokio::sync::Mutex;
use tower_sessions::Session;

use crate::handlers::WebSockets;
use crate::models::{Boggle, PlayerIdSubmission};
use crate::render::Render;

pub struct Handle {}

impl Handle {
    pub async fn root(session: Session) -> impl IntoResponse {
        session.insert("username", "test").await.unwrap();
        println!("\n{:?}", session);
        Html(Render::root()).into_response()
    }

    pub async fn new_game(Extension(boggle): Extension<Arc<Mutex<Boggle>>>) -> impl IntoResponse {
        let mut boggle = boggle.lock().await;

        boggle.new_game().await; // Reset the game state
        (StatusCode::NO_CONTENT, ())
    }

    pub async fn get_player_score(
        Extension(boggle): Extension<Arc<Mutex<Boggle>>>,
        Form(PlayerIdSubmission { username }): Form<PlayerIdSubmission>,
    ) -> impl IntoResponse {
        let boggle = boggle.lock().await;
        let player_score_html = boggle.get_player_score(username).await;

        Html(player_score_html).into_response()
    }

    pub async fn websocket(
        ws: WebSocketUpgrade,
        State(state): State<Arc<Mutex<Boggle>>>,
        session: Session,
    ) -> impl IntoResponse {
        print!("\nFrom Websocket: {:?}", session);
        ws.on_upgrade(|socket| async move { WebSockets::new(socket, state, session).await })
    }
}
