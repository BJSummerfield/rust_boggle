pub mod boggle_render {
    use crate::boggle::BoggleBoard;
    use crate::player_state::PlayerState;
    use maud::{html, PreEscaped};
    use std::collections::HashMap; // Import your BoggleBoard definition

    pub fn render_timer(time_remaining: &str) -> String {
        html! {
            div id="game_timer" {
                (time_remaining)
            }
        }
        .into_string()
    }

    pub fn render_new_user() -> String {
        html! {

            div id = "game_timer" {}
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

    fn render_new_game_button() -> String {
        html! {
            form hx-post="/new_game" {
                button type="submit" { "New Game" }
            }
        }
        .into_string()
    }

    pub fn render_word_input() -> String {
        html! {
            input type="text"
            name="word"
            placeholder="Enter word"
            ws-send
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

    pub fn render_starting_state() -> String {
        html! {
            div id = "game_timer" {
                (PreEscaped(render_new_game_button()))
            }
            div id="game-board" {}
            div id="word-input" {}
            div id="valid-words" {}

        }
        .into_string()
    }

    pub fn render_inprogress_state(timer: &str, board: &BoggleBoard) -> String {
        html! {
            div id="game_timer" {
            (timer)
            }
            div id="game-board" {
                (PreEscaped(render_board(&board)))
            }
            div id="word-input" {
                (PreEscaped(render_word_input()))
            }
            div id="valid-words" {}
        }
        .into_string()
    }

    pub fn render_word_submit(found_words: &[String]) -> String {
        html! {
            div id="word-input" {
                (PreEscaped(render_word_input()))
            }
            div id="valid-words" {
                ul {
                   @for word in found_words {
                       li {
                           div class="word-container" {
                               span class="word" { (word) }
                               span clas="definition" {}
                           }
                       }
                   }
                }
            }

        }
        .into_string()
    }

    pub fn render_gameover_state(
        board: &BoggleBoard,
        players: &HashMap<String, PlayerState>,
    ) -> String {
        // Sort players by score in descending order
        let mut sorted_players: Vec<_> = players.iter().collect();
        sorted_players.sort_by(|a, b| b.1.score.cmp(&a.1.score));

        html! {
            div id="game_timer" {
                (PreEscaped(render_new_game_button()))
            }
            div id="game-board" {
                (PreEscaped(render_board(&board)))
            }
            div id="word-input" {
                (PreEscaped(render_player_scores(&board, &sorted_players)))
            }
            div id="valid-words" {
                (PreEscaped(render_valid_words(&board.valid_words)))
            }
        }
        .into_string()
    }

    fn render_player_scores(
        board: &BoggleBoard,
        sorted_players: &[(&String, &PlayerState)],
    ) -> String {
        html! {
            (PreEscaped(render_scores("Board Total".to_string(), board.total_score.to_string())))
            @for (player_name, player) in sorted_players {
                (PreEscaped(render_scores(player_name.to_string(), player.score.to_string())))
            }
        }
        .into_string()
    }

    fn render_scores(name: String, score: String) -> String {
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

    fn render_board(board: &BoggleBoard) -> String {
        html! {
            @for row in &board.board {
                @for &letter in row {
                    div class="board-cell" {
                        (letter)
                    }
                }
            }
        }
        .into_string()
    }

    pub fn render_valid_words(word_list: &Vec<(String, String)>) -> String {
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
}
