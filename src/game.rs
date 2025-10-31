use crate::ai::AIEngine;
use crate::board::Board;
use crate::game_logger::GameLogger;
use crate::player::{Player, PlayerType, Role};
use crate::terminal_ui::{GameAction, TerminalUI};
use log::{info, warn};
use std::thread;
use std::time::Duration;

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
  pub last_stone_x: Option<usize>, // Coordinates of the last placed stone
  pub last_stone_y: Option<usize>,

  // UI
  pub ui: TerminalUI,

  pub current_role: Role,
  pub round: i32,
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
      cursor_x: size / 2,
      cursor_y: size / 2,
      last_stone_x: None,
      last_stone_y: None,

      ui,

      current_role: p1.role,
      round: 1,
    }
  }

  pub fn run(&mut self) {
    // Initial screen setup
    self.ui.init_screen().unwrap();

    let mut paused = false;
    let mut game_is_over = false;

    loop {
      // Determine the current player based on the current role
      let player = if self.current_role == self.player1.role {
        &self.player1
      } else {
        &self.player2
      };

      // Update the drawing (center the board and draw)
      self.ui.draw_board(
        &self.board,
        self.cursor_x,
        self.cursor_y,
        self.last_stone_x,
        self.last_stone_y,
        player.player_type,
      );

      if paused || game_is_over {
        // If paused or game is over, wait for input
        let action = self.ui.read_input();
        if game_is_over {
          break;
        }
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
            warn!("Error: mode HumanvHuman, but playerType=AI?");
            break;
          }
          // make AI move
          info!("AI is making a move");
          self.turn(player.player_type);
        }
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

            // Move cursor left
            GameAction::MoveLeft => {
              if self.cursor_x > 0 {
                self.cursor_x -= 1;
              }
            }
            // Move cursor right
            GameAction::MoveRight => {
              if self.cursor_x + 1 < self.board.size {
                self.cursor_x += 1;
              }
            }
            // Move cursor up
            GameAction::MoveUp => {
              if self.cursor_y > 0 {
                self.cursor_y -= 1;
              }
            }
            // Move cursor down
            GameAction::MoveDown => {
              if self.cursor_y + 1 < self.board.size {
                self.cursor_y += 1;
              }
            }

            // Place stone (Enter / Space)
            GameAction::PlaceStone => {
              if self.board.board[self.cursor_x + 1][self.cursor_y + 1] == 0 {
                // Check if the cell is free
                info!("Human is placing a stone");
                self.turn(player.player_type);
              }
            }

            GameAction::None => {
              // do nothing
              continue;
            }
          }
        }
      } // end match

      // After making a move, check if the game is over
      if self.board.is_game_over() {
        let w = self.board.get_winner();
        self.print_winner(w);
        game_is_over = true;
      }
    }

    // At the end — restore the terminal to normal state
    self.ui.restore_terminal().unwrap();
  }

  fn turn(&mut self, player_type: PlayerType) {
    match player_type {
      PlayerType::AI => self.ai_turn(),
      PlayerType::Human => self.human_turn(),
    }

    // Switch turn
    self.current_role = self.current_role.opponent();
    self.round += 1;
  }

  fn human_turn(&mut self) {
    self.board.put(self.cursor_x, self.cursor_y, self.current_role);
    self.last_stone_x = Some(self.cursor_x);
    self.last_stone_y = Some(self.cursor_y);
  }

  fn ai_turn(&mut self) {
    let (value, move_xy, _path) = if self.current_role == self.player1.role {
      self.ai1.make_move(&mut self.board, self.current_role)
    } else {
      self.ai2.make_move(&mut self.board, self.current_role)
    };
    let msg = format!("AI ({:?}) chose move with score={}", self.current_role, value);
    self.ui.show_message(&msg);
    info!("AI moved to {:?}", move_xy);
    if let Some((x, y)) = move_xy {
      self.board.put(x, y, self.current_role);
      self.last_stone_x = Some(x);
      self.last_stone_y = Some(y);
    } else {
      self.ui.show_message("AI chose no move");
    }
  }

  fn print_winner(&mut self, w: i32) {
    if w == 0 {
      self.ui.show_message("Game over. Draw!");
    } else if w > 0 {
      self.ui.show_message("0 wins!");
    } else {
      self.ui.show_message("X wins!");
    }
  }

  pub fn run_with_logging(&mut self) {
    let mut logger = GameLogger::new("gomoku_game.log").expect("Failed to create log file");

    println!("Starting AI vs AI game with logging...");
    println!("Log file: gomoku_game.log");
    println!("Board size: {}", self.board.size);
    println!("AI depth: {}", self.player1.depth);
    println!();

    loop {
      let player = if self.current_role == self.player1.role {
        &self.player1
      } else {
        &self.player2
      };

      logger.log_move_start(self.current_role, self.round).ok();
      logger.log_board_state(&self.board).ok();

      println!("Move #{} - {:?} thinking...", self.round, self.current_role);

      match player.player_type {
        PlayerType::AI => {
          self.ai_turn_with_logging(&mut logger);
        }
        PlayerType::Human => {
          panic!("Log mode only supports AI vs AI");
        }
      }

      // Check game over
      if self.board.is_game_over() {
        let winner = self.board.get_winner();
        logger.log_board_state(&self.board).ok();
        logger.log_game_end(winner, self.round).ok();

        println!("\nGame Over!");
        match winner {
          0 => println!("Result: DRAW"),
          1 => println!("Result: WHITE (O) WINS!"),
          -1 => println!("Result: BLACK (X) WINS!"),
          _ => println!("Result: UNKNOWN"),
        }
        println!("Total moves: {}", self.round);
        println!("\nSee gomoku_game.log for detailed analysis.");
        break;
      }

      // Switch turn
      self.current_role = self.current_role.opponent();
      self.round += 1;

      // Small delay for readability
      thread::sleep(Duration::from_millis(100));
    }
  }

  fn ai_turn_with_logging(&mut self, logger: &mut GameLogger) {
    let ai = if self.current_role == self.player1.role {
      &mut self.ai1
    } else {
      &mut self.ai2
    };

    // Get candidates before make_move
    let candidates = self.board.get_valuable_moves(self.current_role, 0, false, false);
    logger.log_candidates(&candidates, self.current_role).ok();

    // IMPORTANT: Use make_move which includes threat detection logic
    let (final_value, final_move, _final_path) = ai.make_move(&mut self.board, self.current_role);

    // Determine reason based on value
    let reason = if final_value >= 10_000_000 {
      "Winning move (FIVE)"
    } else if final_value >= crate::ai::HIGH_VALUE {
      "VCT WIN"
    } else if final_value >= 2_000_000 {
      "Strong attack or critical defense"
    } else if final_value < 0 {
      "Defensive/forced move"
    } else {
      "Standard full-depth search result"
    };

    if let Some((x, y)) = final_move {
      logger.log_patterns(x, y, self.current_role, &self.board).ok();
      logger.log_final_decision(final_move, final_value, reason).ok();
      logger
        .log_cache_stats(ai.cache_hits.hit, ai.cache_hits.total, ai.cache_hits.search)
        .ok();

      println!("  -> Move: ({}, {}) Score: {} [{}]", x, y, final_value, reason);

      // Place the stone on the board
      self.board.put(x, y, self.current_role);
      self.last_stone_x = Some(x);
      self.last_stone_y = Some(y);
    } else {
      logger.log_final_decision(None, final_value, "No valid moves found").ok();
      println!("  -> No valid moves");
    }
  }
}
