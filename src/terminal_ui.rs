use crossterm::{
  execute,
  terminal::{EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode, disable_raw_mode, size},
  cursor::{Hide, Show, MoveTo},
  event::{read, Event, KeyEvent, KeyCode, KeyModifiers},
  style::Print,
};

use std::io::{stdout, Write, Result as IoResult}; // Note, we take Result as IoResult
use crate::board::Board;

// Definition of the GameAction enum for various actions in the game
#[derive(Debug)]
pub enum GameAction {
  None,          // No action
  Quit,          // Quit the game
  TogglePause,   // Toggle pause
  Undo,          // Undo action
  Redo,          // Redo action
  MoveLeft,      // Move left
  MoveRight,     // Move right
  MoveUp,        // Move up
  MoveDown,      // Move down
  PlaceStone,    // Place stone
}

// Structure for the terminal user interface
pub struct TerminalUI;

impl TerminalUI {
  // Constructor for creating a new instance of TerminalUI
  pub fn new() -> Self {
      Self
  }

  // Initialization of the terminal screen
  pub fn init_screen(&mut self) -> IoResult<()> {
      enable_raw_mode()?; // Enable raw input mode
      execute!(stdout(), EnterAlternateScreen, Hide)?; // Enter alternate screen and hide cursor
      Ok(())
  }

  // Restore the terminal state
  pub fn restore_terminal(&mut self) -> IoResult<()> {
      execute!(stdout(), Show, LeaveAlternateScreen)?; // Show cursor and leave alternate screen
      disable_raw_mode()?; // Disable raw input mode
      Ok(())
  }

  // Read user input and determine the action
  pub fn read_input(&mut self) -> GameAction {
      if let Ok(ev) = read() { // Read event
          match ev {
              Event::Key(KeyEvent { code, .. }) => { // Handle key event
                  match code {
                      KeyCode::Esc | KeyCode::Char('q') => {
                          return GameAction::Quit; // Quit the game
                      }
                      KeyCode::Char('p') => {
                          return GameAction::TogglePause; // Toggle pause
                      }
                      KeyCode::Backspace => {
                          return GameAction::Undo; // Undo action
                      }
                      KeyCode::Tab => {
                          return GameAction::Redo; // Redo action
                      }
                      KeyCode::Left => return GameAction::MoveLeft, // Move left
                      KeyCode::Right => return GameAction::MoveRight, // Move right
                      KeyCode::Up => return GameAction::MoveUp, // Move up
                      KeyCode::Down => return GameAction::MoveDown, // Move down
                      KeyCode::Enter | KeyCode::Char(' ') => {
                          return GameAction::PlaceStone; // Place stone
                      }
                      _ => {}
                  }
              }
              _ => {}
          }
      }
      GameAction::None // No action
  }

  // Display a message at the bottom of the screen
  pub fn show_message(&mut self, msg: &str) {
      let (_cols, rows) = size().unwrap_or((80, 24)); // Get terminal size
      let y = rows.saturating_sub(1); // Determine the row for message display
      execute!(stdout(), MoveTo(0, y), Print(" ".repeat(80))).ok(); // Clear the line
      let (_cols, rows) = size().unwrap_or((80, 24));
      let y = rows.saturating_sub(1);
      execute!(stdout(), MoveTo(0, y), Print(" ".repeat(80))).ok();
      execute!(stdout(), MoveTo(0, y), Print(msg)).ok();
  }

  pub fn draw_board(&mut self, board: &Board, cursor_x: usize, cursor_y: usize) {
      let (cols, rows) = size().unwrap_or((80, 24));

      let bsize = board.size as u16;
      let cell_width: u16 = 2;
      let used_width = bsize * cell_width;
      let used_height = bsize;

      let offset_x = if cols > used_width {
          (cols - used_width) / 2
      } else {
          0
      };
      let offset_y = if rows > used_height {
          (rows - used_height) / 2
      } else {
          0
      };

      let mut stdout_ = stdout();

      // Clear the screen
      for row in 0..rows {
          execute!(stdout_, MoveTo(0, row), Print(" ".repeat(cols as usize))).ok();
      }

      // Draw cells
      for i in 0..board.size {
          for j in 0..board.size {
              let cell = board.board[i][j];
              let ch = match cell {
                  1 => "O",
                  -1 => "X",
                  _ => ".",
              };
              let sx = offset_x + (j as u16) * cell_width;
              let sy = offset_y + (i as u16);

              execute!(stdout_, MoveTo(sx, sy), Print(ch)).ok();
          }
      }

      // Draw cursor
      if cursor_x < board.size && cursor_y < board.size {
          let cur_sx = offset_x + (cursor_x as u16) * cell_width;
          let cur_sy = offset_y + (cursor_y as u16);
          execute!(stdout_, MoveTo(cur_sx, cur_sy), Print("+")).ok();
      }
  }
}
