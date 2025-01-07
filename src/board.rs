use crate::player::Role;

pub struct Board {
  pub size: usize,
  pub board: Vec<Vec<i32>>, // 0=empty, +1=white, -1=black
  pub history: Vec<(usize, usize, Role)>, // History of moves
}

impl Board {
  // Create a new board with the given size
  pub fn new(size: usize) -> Self {
    let b = vec![vec![0; size]; size]; // Initialize the board with zeros
    Self {
      size,
      board: b,
      history: Vec::new(), // Initialize an empty history
    }
  }

  // Place a stone on the board
  pub fn put(&mut self, i: usize, j: usize, role: Role) -> bool {
    if i >= self.size || j >= self.size { // Check if the position is out of bounds
      return false;
    }
    if self.board[i][j] != 0 { // Check if the position is already occupied
      return false;
    }
    self.board[i][j] = role.to_int(); // Place the stone
    self.history.push((i, j, role)); // Record the move in history
    true
  }

  // Undo the last move
  pub fn undo(&mut self) -> bool {
    match self.history.pop() { // Remove the last move from history
      None => false, // No move to undo
      Some((i, j, _role)) => {
        self.board[i][j] = 0; // Clear the position on the board
        true
      }
    }
  }

  // Check if the game is over
  pub fn is_game_over(&self) -> bool {
    if self.get_winner() != 0 { // Check if there is a winner
      return true;
    }
    for i in 0..self.size {
      for j in 0..self.size {
        if self.board[i][j] == 0 { // Check if there are empty positions
          return false;
        }
      }
    }
    true // No empty positions, game is over
  }

  // Get the winner of the game
  pub fn get_winner(&self) -> i32 {
    // Simplified: no full logic.
    // In a real project, search for 5 in a row.
    0
  }

  // Evaluate the board for a given role
  pub fn evaluate(&self, _role: Role) -> i32 {
    // Simplified stub
    0
  }

  // Generate a hash for the board state
  pub fn hash(&self) -> u64 {
    // Stub
    0
  }
}
