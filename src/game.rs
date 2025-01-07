use crate::ai::AIEngine;
use crate::board::Board;
use crate::player::{Player, PlayerType, Role};
use crate::terminal_ui::{GameAction, TerminalUI};

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
      // 1) Update the drawing (center the board and draw)
      self.ui.draw_board(
        &self.board,
        self.cursor_x,
        self.cursor_y,
        self.last_stone_x,
        self.last_stone_y,
      );

      // 2) Depending on the mode and the current player, make a move
      let player = if self.current_role == self.player1.role {
        &self.player1
      } else {
        &self.player2
      };

      if paused || game_is_over {
        // If paused, wait for input
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
            println!("Error: mode HumanvHuman, but playerType=AI?");
            break;
          }
          // make AI move
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
              if self.board.board[self.cursor_y + 1][self.cursor_x + 1] == 0 {
                // Check if the cell is free
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

      // После совершения хода — проверяем окончание игры
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

    self.last_stone_x = Some(self.cursor_x);
    self.last_stone_y = Some(self.cursor_y);

    // Переход хода
    self.current_role = self.current_role.opponent();
    self.round += 1;

    // Сохраняем сообщение во временную переменную
    // self.ui.show_message(&format!("Round: {}", self.round));
  }

  fn human_turn(&mut self) {
    self.board.put(self.cursor_y, self.cursor_x, self.current_role);
  }

  fn ai_turn(&mut self) {
    let (value, move_xy, _path) = if self.current_role == self.player1.role {
      self.ai1.make_move(&mut self.board, self.current_role)
    } else {
      self.ai2.make_move(&mut self.board, self.current_role)
    };
    let msg = format!("AI ({:?}) chose move with score={}", self.current_role, value);
    self.ui.show_message(&msg);
    if let Some((r, c)) = move_xy {
      self.board.put(r, c, self.current_role);
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
}
