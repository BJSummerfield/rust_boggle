use axum::extract::{
    ws::{Message, WebSocket, WebSocketUpgrade},
    State,
};
use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Extension, Router,
};
use maud::html;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tower_http::services::ServeDir;

use futures::{sink::SinkExt, stream::StreamExt};

mod boggle;
use boggle::BoggleBoard;

mod dictionary;
use dictionary::Dictionary;
mod gamestate;
use gamestate::GameState;

#[tokio::main]
async fn main() {
    // Set up the router

    let game_state = Arc::new(Mutex::new(GameState::new()));
    GameState::start_timer(game_state.clone());

    let file_path = format!(
        "{}/static/scrabble-dictionary.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    let dictionary = Arc::new(Dictionary::new(&file_path).expect("Failed to create dictionary"));

    let app = Router::new()
        .route("/", get(serve_boggle_board))
        .layer(Extension(dictionary))
        // Serve static files from the `static` directory
        .nest_service("/static", ServeDir::new("static"))
        .route("/ws", get(websocket_handler))
        .with_state(game_state);

    // Bind to a socket address
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    // Run the server
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<Mutex<GameState>>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}
// create a websocket function for handling sending websocket messages to the clients
async fn websocket(ws: WebSocket, state: Arc<Mutex<GameState>>) {
    let (mut tx, _) = ws.split();
    // Subscribe to the broadcast channel
    let mut rx = {
        let state_locked = state.lock().unwrap_or_else(|e| e.into_inner());
        state_locked.tx.subscribe()
    };
    // Loop over messages received on the broadcast channel
    while let Ok(msg) = rx.recv().await {
        // Send the message to the client
        if tx.send(Message::Text(msg)).await.is_err() {
            break;
        }
    }
}

async fn serve_boggle_board(Extension(dictionary): Extension<Arc<Dictionary>>) -> Html<String> {
    let boggle_board = BoggleBoard::new(&dictionary);
    let markup = html! {
        (maud::DOCTYPE)
        html {
            head {
                title { "Boggle Game" }
                script
                    src="https://unpkg.com/htmx.org@1.9.9"
                    integrity="sha384-QFjmbokDn2DjBjq+fM+8LUIVrAgqcNW2s0PjAxHETgRn9l4fvX31ZxDxvwQnyMOX"
                    crossorigin="anonymous" {}
                script src="https://unpkg.com/htmx.org/dist/ext/ws.js" {}
               link rel="stylesheet" href="/static/style.css";
            }
            body {
                h1 { "Boggle Game" }
                div hx-ext="ws" ws-connect="/ws" {}
                div id="game_timer" {}
                div id="game-board" {
                    // Render the Boggle board
                    @for row in &boggle_board.board {
                        div class="board-row" {
                            @for &letter in row {
                                div class="board-cell" {
                                    (letter)
                                }
                            }
                        }
                    }
                }
            }
            div id="valid-words" {
                ul {
                    @for (word, definition) in &boggle_board.valid_words {
                        li {
                            div class="word-container" {
                                span class="word" { (word) }
                                span class="definition" { (definition) }
                            }
                        }
                    }
                }
            }
        }            // Add more HTML for the input form and other game controls
    };

    Html(markup.into_string())
}
