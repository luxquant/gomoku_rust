use crate::board::Board;
use crate::player::{Player, PlayerType, Role};
use crate::ai::AIEngine;
use crate::terminal_ui::{TerminalUI, GameAction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    AIvAI,
    AIvHuman,
    HumanvHuman,
}

pub struct Game {
    pub board: Board,
    pub mode: GameMode,

    pub player1: Player,
    pub player2: Player,

    pub ai1: AIEngine,
    pub ai2: AIEngine,

    // Current position "cursor" for human move
    pub cursor_x: usize,
    pub cursor_y: usize,

    // UI
    pub ui: TerminalUI,
}

impl Game {
    pub fn new(size: usize, mode: GameMode, p1: Player, p2: Player) -> Self {
        let board = Board::new(size);
        let ai1 = AIEngine::new(p1.depth);
        let ai2 = AIEngine::new(p2.depth);

        let ui = TerminalUI::new();

        Self {
            board,
            mode,
            player1: p1,
            player2: p2,
            ai1,
            ai2,
            cursor_x: 0,
            cursor_y: 0,
            ui,
        }
    }

    pub fn run(&mut self) {
        // Initial screen setup
        self.ui.init_screen().unwrap();

        let mut current_role = self.player1.role;
        let mut round = 1;
        let mut paused = false;

        loop {
            // 1) Check if the game is over
            if self.board.is_game_over() {
                let w = self.board.get_winner();
                self.print_winner(w);
                break;
            }

            // 2) Update the drawing (center the board and draw)
            self.ui.draw_board(&self.board, self.cursor_x, self.cursor_y);

            // 3) Depending on the mode and the current player, make a move
            let player = if current_role == self.player1.role {
                &self.player1
            } else {
                &self.player2
            };

            if paused {
                // If paused, wait for input
                let action = self.ui.read_input();
                match action {
                    GameAction::Quit => break,
                    GameAction::TogglePause => {
                        paused = false; // unpause
                    }
                    GameAction::None => {
                        // do nothing
                    }
                    // additional rewind logic can be added here
                    _ => {}
                }
                continue; // skip player switch
            }

            match player.player_type {
                PlayerType::AI => {
                    if self.mode == GameMode::HumanvHuman {
                        // protection against mismatch
                        println!("Error: mode HumanvHuman, but playerType=AI?");
                        break;
                    }
                    // make AI move
                    self.ai_turn(current_role);
                },
                PlayerType::Human => {
                    // handle player input: arrows, backspace, tab, P, enter, etc.
                    let action = self.ui.read_input();

                    match action {
                        // Pause
                        GameAction::TogglePause => {
                            paused = !paused;
                            continue;
                        }

                        // Quit game (Esc, Q)
                        GameAction::Quit => {
                            break;
                        }

                        // Undo move
                        GameAction::Undo => {
                            if !self.board.undo() {
                                self.ui.show_message("No moves to undo.");
                            }
                            continue; 
                        }

                        // Redo move (simplified, not implemented — need to store future moves)
                        GameAction::Redo => {
                            // logic here if we stored future moves
                            continue;
                        }

                        // Move cursor
                        GameAction::MoveLeft => {
                            if self.cursor_x > 0 {
                                self.cursor_x -= 1;
                            }
                        }
                        GameAction::MoveRight => {
                            if self.cursor_x + 1 < self.board.size {
                                self.cursor_x += 1;
                            }
                        }
                        GameAction::MoveUp => {
                            if self.cursor_y > 0 {
                                self.cursor_y -= 1;
                            }
                        }
                        GameAction::MoveDown => {
                            if self.cursor_y + 1 < self.board.size {
                                self.cursor_y += 1;
                            }
                        }

                        // Place stone (Enter / Space)
                        GameAction::PlaceStone => {
                            // Check if the cell is free
                            if self.board.board[self.cursor_y][self.cursor_x] == 0 {
                                self.board.put(self.cursor_y, self.cursor_x, current_role);
                            } else {
                                self.ui.show_message("Cell is occupied!");
                                continue;
                            }
                        }

                        GameAction::None => {
                            // do nothing
                            continue;
                        }
                    }
                }
            } // end match

            // 4) After making a move — check if the game is over
            if self.board.is_game_over() {
                let w = self.board.get_winner();
                self.print_winner(w);
                break;
            }

            // 5) Switch turn
            current_role = current_role.opponent();
            round += 1;
        }

        // At the end — restore the terminal to normal state
        self.ui.restore_terminal().unwrap();
    }

    fn ai_turn(&mut self, role: Role) {
        let (value, move_xy, _path) = if role == self.player1.role {
            self.ai1.search_move(&mut self.board, role, self.ai1.depth)
        } else {
            self.ai2.search_move(&mut self.board, role, self.ai2.depth)
        };
        let msg = format!("AI ({:?}) chose move with score={}", role, value);
        self.ui.show_message(&msg);
        if let Some((r, c)) = move_xy {
            self.board.put(r, c, role);
        }
    }

    fn print_winner(&mut self, w: i32) {
        if w == 0 {
            self.ui.show_message("Game over. Draw!");
        } else if w > 0 {
            self.ui.show_message("White wins!");
        } else {
            self.ui.show_message("Black wins!");
        }
    }
}
