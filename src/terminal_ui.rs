use crossterm::{
  cursor::{Hide, MoveTo, Show},
  event::{read, Event, KeyCode, KeyEvent},
  execute,
  style::{Color, Print, ResetColor, SetForegroundColor},
  terminal::{disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::board::Board;
use std::io::{stdout, Result as IoResult}; // Note, we take Result as IoResult

// Definition of the GameAction enum for various actions in the game
#[derive(Debug)]
pub enum GameAction {
  None,        // No action
  Quit,        // Quit the game
  TogglePause, // Toggle pause
  Undo,        // Undo action
  Redo,        // Redo action
  MoveLeft,    // Move left
  MoveRight,   // Move right
  MoveUp,      // Move up
  MoveDown,    // Move down
  PlaceStone,  // Place stone
}

// Structure for the terminal user interface
pub struct TerminalUI {
  /// Store the last message to be displayed on the bottom line.
  last_message: String,
}

impl TerminalUI {
  /// "Light green" for the cursor, RGB value
  const CURSOR_COLOR: Color = Color::Rgb { r: 120, g: 255, b: 120 };
  /// "Light red" for the last stone, RGB value
  const LAST_STONE_COLOR: Color = Color::Rgb { r: 255, g: 140, b: 140 };

  // Constructor for creating a new instance of TerminalUI
  pub fn new() -> Self {
    Self {
      last_message: String::new(), // Initially an empty string
    }
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
    if let Ok(ev) = read() {
      // Read event
      match ev {
        Event::Key(KeyEvent { code, .. }) => {
          // Handle key event
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
            KeyCode::Left => return GameAction::MoveLeft,   // Move left
            KeyCode::Right => return GameAction::MoveRight, // Move right
            KeyCode::Up => return GameAction::MoveUp,       // Move up
            KeyCode::Down => return GameAction::MoveDown,   // Move down
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

  /// Set (and immediately draw) a new message
  pub fn show_message(&mut self, msg: &str) {
    // Save to the field
    self.last_message = msg.to_string();
    // Draw
    self.draw_message();
  }

  /// Actually output `self.last_message` on the bottom line
  fn draw_message(&mut self) {
    let (cols, rows) = size().unwrap_or((80, 24));
    let y = rows.saturating_sub(2); // Print the message on the line above

    // Center the message
    let msg_len = self.last_message.len() as u16;
    let x = if cols > msg_len { (cols - msg_len) / 2 } else { 0 };

    // Clear the line (cols number of spaces)
    execute!(stdout(), MoveTo(0, y), Print(" ".repeat(cols as usize))).ok();
    // Print the message
    execute!(stdout(), MoveTo(x, y), Print(&self.last_message)).ok();
  }

  pub fn draw_board(
    &mut self,
    board: &Board,
    cursor_x: usize,
    cursor_y: usize,
    last_stone_x: Option<usize>,
    last_stone_y: Option<usize>,
  ) {
    let (cols, rows) = size().unwrap_or((80, 24));

    let bsize = board.size as u16;
    let cell_width: u16 = 3; // Увеличиваем ширину ячейки для добавления пробела
    let used_width = bsize * cell_width - 1;
    let used_height = bsize;

    // Calculate offsets for centering
    let offset_x = if cols > used_width { (cols - used_width) / 2 } else { 0 };
    let offset_y = if rows > used_height { (rows - used_height) / 2 } else { 0 };

    let mut stdout_ = stdout();

    // Clear only the part where the board will be (optional: can clear the entire screen)
    for row in 0..rows {
      execute!(stdout_, MoveTo(0, row), Print(" ".repeat(cols as usize))).ok();
    }

    // Draw top border with special characters
    execute!(stdout_, MoveTo(offset_x, offset_y - 1), Print("╔")).ok();
    for _ in 0..used_width {
      execute!(stdout_, Print("═")).ok();
    }
    execute!(stdout_, Print("╗")).ok();

    // Draw cells with side borders
    for i in 1..=board.size {
      execute!(stdout_, MoveTo(offset_x, offset_y + (i as u16) - 1), Print("║")).ok();
      for j in 1..=board.size {
        let stone = board.board[i][j]; // 1=O, -1=X, 0=empty
                                       // Determine if coloring is needed
        let sx = offset_x + ((j - 1) as u16) * cell_width + 1;
        let sy = offset_y + ((i - 1) as u16);

        // Check if this position is the last placed stone
        let is_last_stone = if let (Some(lx), Some(ly)) = (last_stone_x, last_stone_y) {
          lx == j - 1 && ly == i - 1
        } else {
          false
        };

        // Check if the cursor is here
        let is_cursor = (j - 1 == cursor_x) && (i - 1 == cursor_y);

        // We will print either 'X', 'O', or '.'.
        // But if the cursor is on an occupied cell, we need to "highlight" the figure.
        // If the cursor is on an empty cell, we place a "+".
        let (symbol, color) = match stone {
          1 => {
            // Stone 'O'
            if is_cursor {
              // Hovered over O => make "O" green
              ("O", Some(Self::CURSOR_COLOR))
            } else if is_last_stone {
              // Last stone 'O'
              ("O", Some(Self::LAST_STONE_COLOR))
            } else {
              // Regular O (white or no special color)
              ("O", None)
            }
          }
          -1 => {
            // Stone 'X'
            if is_cursor {
              // Hovered over X => make "X" green
              ("X", Some(Self::CURSOR_COLOR))
            } else if is_last_stone {
              ("X", Some(Self::LAST_STONE_COLOR))
            } else {
              ("X", None)
            }
          }
          0 => {
            // Empty cell
            if is_cursor {
              // Cursor here => plus sign in green
              ("+", Some(Self::CURSOR_COLOR))
            } else {
              // Just "."
              (".", None)
            }
          }
          _ => ("?", None), // just in case
        };

        // Print
        if let Some(col) = color {
          // Set the required color, print the symbol, reset the color
          execute!(stdout_, MoveTo(sx, sy), SetForegroundColor(col), Print(symbol), ResetColor).ok();
        } else {
          // Without color
          execute!(stdout_, MoveTo(sx, sy), Print(symbol)).ok();
        }
        // Добавляем пробел между ячейками
        execute!(stdout_, Print(" ")).ok();
      }
      execute!(stdout_, Print("║")).ok();
    }

    // Draw bottom border with special characters
    execute!(stdout_, MoveTo(offset_x, offset_y + used_height), Print("╚")).ok();
    for _ in 0..used_width {
      execute!(stdout_, Print("═")).ok();
    }
    execute!(stdout_, Print("╝")).ok();

    // After drawing the board – output the saved message again
    // (so that the line is not overwritten)
    self.draw_message();
  }
}
