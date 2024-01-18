use axum::{
    extract::{ws::WebSocketUpgrade, Form, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    Extension,
};
use uuid::Uuid;

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use tower_sessions::Session;

use crate::models::{Boggle, PlayerIdSubmission};
use crate::render::Render;
use crate::{handlers::WebSockets, models::PlayerId};
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize, Debug)]
pub struct User {
    pub username: String,
}

#[derive(Deserialize, Debug)]
pub struct WordSubmission {
    word: String,
}
pub struct Handle {}

impl Handle {
    pub async fn root(session: Session) -> impl IntoResponse {
        Self::update_last_seen(&session).await;

        if session.get::<String>("id").await.unwrap_or(None).is_none() {
            // Generate a new UUID and store it in the session
            let new_id = Uuid::new_v4().to_string();
            session
                .insert("id", &new_id)
                .await
                .expect("Failed to insert new ID into session");
        }

        match session.get::<String>("username").await {
            Ok(Some(username)) => {
                println!("Username found in session: {}", username);
                Html(Render::root()).into_response()
            }
            _ => Html(Render::root_no_username()).into_response(),
        }
    }

    pub async fn username(
        session: Session,
        Form(PlayerIdSubmission { username }): Form<PlayerIdSubmission>,
    ) -> impl IntoResponse {
        session
            .insert("username", username)
            .await
            .expect("Could not serialize.");
        println!("\n{:?}", session);
        Html(Render::shell_template()).into_response()
    }

    pub async fn submit_word(
        session: Session,
        Extension(boggle): Extension<Arc<Mutex<Boggle>>>,
        Form(WordSubmission { word }): Form<WordSubmission>,
    ) -> impl IntoResponse {
        let player_id = session
            .get::<PlayerId>("id")
            .await
            .expect("Could not deserialize.")
            .unwrap();

        let mut boggle = boggle.lock().await;
        let word_submission_html = boggle.submit_word(&player_id, &word);

        Html(word_submission_html).into_response()
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
        ws.on_upgrade(|socket| async move { WebSockets::new(socket, state, session).await })
    }

    async fn update_last_seen(session: &Session) {
        let current_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        session
            .insert("last_seen", current_timestamp)
            .await
            .expect("Failed to insert last_seen into session");
    }
}
