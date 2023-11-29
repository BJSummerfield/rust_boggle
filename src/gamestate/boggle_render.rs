pub mod boggle_render {
    use maud::{html, PreEscaped};

    use crate::boggle::BoggleBoard; // Import your BoggleBoard definition

    pub fn render_timer(time_remaining: &str) -> String {
        html! {
            div id="game_timer" {
                (time_remaining)
            }
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

    pub fn render_starting_state() -> String {
        html! {
            div id = "game_timer" {
                (PreEscaped(render_new_game_button()))
            }
            div id="game-board" {}
            div id="valid-words" {}
        }
        .into_string()
    }

    pub fn render_inprogress_state(timer: &str, board: &Option<BoggleBoard>) -> String {
        html! {
            div id="game_timer" {
            (timer)
            }
            div id="game-board" {
            (PreEscaped(render_board(&board)))
            }
            div id="valid-words" {}
        }
        .into_string()
    }

    pub fn render_gameover_state(board: &Option<BoggleBoard>) -> String {
        html! {
            div id="game_timer" {
                (PreEscaped(render_new_game_button()))
            }
            div id="game-board" {
                (PreEscaped(render_board(&board)))
            }
            div id="valid-words" {
                (PreEscaped(render_valid_words(board)))
            }
        }
        .into_string()
    }

    fn render_board(board_option: &Option<BoggleBoard>) -> String {
        html! {
            @if let Some(board) = board_option {
                @for row in &board.board {
                    // div class="board-row" {
                        @for &letter in row {
                            div class="board-cell" {
                                (letter)
                            }
                        }
                    // }
                }
            }
        }
        .into_string()
    }

    fn render_valid_words(board_option: &Option<BoggleBoard>) -> String {
        html! {
        @if let Some(ref board) = board_option {
               ul {
                       @for (word, definition) in &board.valid_words {
                           li {
                               div class="word-container" {
                                   span class="word" { (word) }
                                   span class="definition" { (definition) }
                               }
                           }
                       }
                   }
               }
        }
        .into_string()
    }
}