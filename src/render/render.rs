use crate::models::{Board, PlayerList};
use maud::{html, PreEscaped};

pub struct Render {}

impl Render {
    pub fn timer(time_remaining: &str) -> String {
        html! {
            div id="game-timer" {
                (time_remaining)
            }
        }
        .into_string()
    }

    pub fn new_user() -> String {
        html! {

            div id = "game-timer" {}
            div id="game-board" {}
            div id="word-input" {
                    input type="text"
                    name="username"
                    placeholder="Enter username"
                    ws-send
                    required
                    {}
            }
            div id="valid-words" {}
        }
        .into_string()
    }

    fn new_game_button() -> String {
        html! {
            form hx-post="/new_game" {
                button type="submit" { "New Game" }
            }
        }
        .into_string()
    }

    pub fn word_input() -> String {
        html! {
            input type="text"
            name="word"
            placeholder="Enter word"
            hx-post="/submit_word"
            hx-target="#valid-words"
            title="Only alphabetic characters; 2-16 letters."
            maxlength="16"
            minlength="2"
            required
            autofocus
            {}
            script { "document.addEventListener('DOMContentLoaded', function() { document.getElementsByName('word')[0].focus(); });" }
        }
        .into_string()
    }

    pub fn starting_state() -> String {
        println!("\n rendering html");
        html! {
            div id = "game-timer" {
                (PreEscaped(Self::new_game_button()))
            }
            div id="game-board" {}
            div id="word-input" {}
            div id="valid-words" {}

        }
        .into_string()
    }

    pub fn inprogress_state(timer: &str, board: &Board) -> String {
        html! {
            div id="game-timer" {
            (timer)
            }
            div id="game-board" {
                (PreEscaped(Self::board(&board)))
            }
            div id="word-input" {
                (PreEscaped(Self::word_input()))
            }
            div id="valid-words" {}
        }
        .into_string()
    }

    pub fn word_submit(found_words: &[String]) -> String {
        html! {
            div id="word-input" {
                (PreEscaped(Self::word_input()))
            }
            div id="valid-words" {
                ul {
                   @for word in found_words {
                       li {
                           div class="word-container" {
                               span class="word" { (word) }
                               // span class="definition" {}
                           }
                       }
                   }
                }
            }

        }
        .into_string()
    }

    pub fn gameover_state(board: &Board, players: &PlayerList) -> String {
        // Sort players by score in descending order
        html! {
            div id="game-timer" {
                (PreEscaped(Self::new_game_button()))
            }
            div id="game-board" {
                (PreEscaped(Self::board(&board)))
            }
            div id="word-input" {
                (PreEscaped(Self::player_scores(&board, &players)))
            }
            div id="valid-words" {
                (PreEscaped(Self::valid_words(&board.valid_words)))
            }
        }
        .into_string()
    }

    fn player_scores(board: &Board, players: &PlayerList) -> String {
        let sorted_players = players.get_players_sorted_by_score();
        html! {
            (PreEscaped(Self::scores("Board Total".to_string(), board.total_score.to_string())))
            @for (player_name, player) in sorted_players {
                (PreEscaped(Self::scores(player_name.to_string(), player.score.to_string())))
            }
        }
        .into_string()
    }

    fn scores(name: String, score: String) -> String {
        html! {
            form action="/get_player_score" method="post" hx-post="/get_score" hx-trigger="click" hx-target="#valid-words" {
                input type="hidden" name="username" value=(name) {}
                div class="player-container"  {
                    (name) ": " (score)
                }
            }
        }
        .into_string()
    }

    fn board(board: &Board) -> String {
        html! {
            @for row in &board.board {
                @for &letter in row {
                    div class="board-cell" {
                        // Check if the letter is 'Q' and display "Qu" instead
                        @if letter == 'Q' {
                            "Qu"
                        } @else {
                            (letter)
                        }
                    }
                }
            }
        }
        .into_string()
    }

    pub fn valid_words(word_list: &Vec<(String, String)>) -> String {
        html! {
           ul {
               @for (word, definition) in word_list {
                   li {
                       div class="word-container" {
                           span class="word" { (word) }
                           span class="definition" { (definition) }
                       }
                   }
               }
           }
        }
        .into_string()
    }

    pub fn root() -> String {
        html! {
            (PreEscaped(Self::render_header()))
            body {
                h1 { "Boggle Game" }
                (PreEscaped(Self::shell_template()))
            }
        }
        .into_string()
    }

    pub fn shell_template() -> String {
        html! {
            (PreEscaped(Self::render_header()))
            div id="game-container" hx-ext="ws" ws-connect="/ws" {
                div id="game-timer" {}
                div id="game-board" {}
                div id="word-input" {}
                div id="valid-words" {}
            }
        }
        .into_string()
    }

    fn render_header() -> String {
        html! {
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
            }
        }.into_string()
    }

    pub fn root_no_username() -> String {
        html! {
            (PreEscaped(Self::render_header()))
            body {
                h1 { "Boggle Game" }
                div id="main-container" {
                    div id="word-input" {
                        (PreEscaped(Self::username_form()))
                    }
                }
            }
        }
        .into_string()
    }

    pub fn username_form() -> String {
        html! {
            form method="post" hx-post="/username" hx-target="#main-container" {
                input type="text"
                name="username"
                placeholder="Enter username"
                maxlength="9"
                required
                autofocus
                {}
                button type="submit" style="display: none;" { "Submit" }
            }

        }
        .into_string()
    }
}
