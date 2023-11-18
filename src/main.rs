use axum::{response::Html, routing::get, Extension, Router};
use maud::html;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::services::ServeDir;

mod boggle;
use boggle::BoggleBoard;

mod dictionary;
use dictionary::Dictionary;

mod gamestate;
use gamestate::{GameState, GameStateEnum};

#[tokio::main]
async fn main() {
    // Set up the router
    let game_state = GameState::new();

    game_state
        .lock()
        .unwrap()
        .update_state(GameStateEnum::InProgress);

    let file_path = format!(
        "{}/static/scrabble-dictionary.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    let dictionary = Arc::new(Dictionary::new(&file_path).expect("Failed to create dictionary"));
    let app = Router::new()
        .route("/", get(serve_boggle_board))
        .layer(Extension(dictionary))
        // Serve static files from the `static` directory
        .nest_service("/static", ServeDir::new("static"));

    // Bind to a socket address
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    // Run the server
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn serve_boggle_board(Extension(dictionary): Extension<Arc<Dictionary>>) -> Html<String> {
    let boggle_board = BoggleBoard::new(&dictionary);
    let markup = html! {
        (maud::DOCTYPE)
        html {
            head {
                title { "Boggle Game" }
                // Include your stylesheet and scripts here
                link rel="stylesheet" href="/static/style.css";
            }
            body {
                h1 { "Boggle Game" }
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

// async fn serve_websocket() -> Html<String> {
//     // Placeholder for the WebSocket handling
//     Html("WebSocket setup will go here.".to_string())
// }
