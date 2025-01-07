use crate::cache::{Cache, CacheConfig};
use crate::player::Role;
use crate::zobrist_cache::ZobristCache;

pub struct Board {
  pub size: usize,
  pub board: Vec<Vec<i32>>,               // 0=empty, +1=white, -1=black
  pub history: Vec<(usize, usize, Role)>, // History of moves
  pub zorbist_cache: ZobristCache,
  pub winner_cache: Cache<u64, i32>,
  pub gameover_cache: Cache<u64, bool>,
}

impl Board {
  // Create a new board with the given size
  pub fn new(size: usize) -> Self {
    let b = vec![vec![0; size]; size]; // Initialize the board with zeros
    Self {
      size,
      board: b,
      history: Vec::new(),                                // Initialize an empty history
      zorbist_cache: ZobristCache::new(size),             // Initialize Zobrist cache for the board size
      winner_cache: Cache::new(CacheConfig::default()),   // Initialize winner cache
      gameover_cache: Cache::new(CacheConfig::default()), // Initialize gameover cache
    }
  }

  // Place a stone on the board
  pub fn put(&mut self, i: usize, j: usize, role: Role) -> bool {
    if i >= self.size || j >= self.size {
      // Check if the position is out of bounds
      return false;
    }
    if self.board[i][j] != 0 {
      // Check if the position is already occupied
      return false;
    }
    self.board[i][j] = role.to_int(); // Place the stone
    self.history.push((i, j, role)); // Record the move in history
    true
  }

  // Undo the last move
  pub fn undo(&mut self) -> bool {
    match self.history.pop() {
      // Remove the last move from history
      None => false, // No move to undo
      Some((i, j, _role)) => {
        self.board[i][j] = 0; // Clear the position on the board
        true
      }
    }
  }

  // Check if the game is over
  pub fn is_game_over(&mut self) -> bool {
    let hash = self.hash();
    if let Some(&val) = self.gameover_cache.get(&hash) {
      if val {
        return true;
      }
    }

    if self.get_winner() != 0 {
      self.gameover_cache.put(hash, true);
      return true;
    }

    for i in 0..self.size {
      for j in 0..self.size {
        if self.board[i][j] == 0 {
          self.gameover_cache.put(hash, false);
          return false;
        }
      }
    }

    self.gameover_cache.put(hash, true);
    true
  }

  // Get the winner of the game
  pub fn get_winner(&mut self) -> i32 {
    let hash = self.hash();
    if let Some(&val) = self.winner_cache.get(&hash) {
      if val != 0 {
        return val;
      }
    }

    let directions = [(1, 0), (0, 1), (1, 1), (1, -1)];
    for i in 0..self.size {
      for j in 0..self.size {
        let cell = self.board[i][j];
        if cell == 0 {
          continue;
        }
        for &(dx, dy) in &directions {
          let mut count = 0;
          while i as isize + dx * count >= 0
            && i as isize + dx * count < self.size as isize
            && j as isize + dy * count >= 0
            && j as isize + dy * count < self.size as isize
            && self.board[(i as isize + dx * count) as usize][(j as isize + dy * count) as usize] == cell
          {
            count += 1;
          }
          if count >= 5 {
            self.winner_cache.put(hash, cell);
            return cell;
          }
        }
      }
    }
    self.winner_cache.put(hash, 0);
    0
  }

  // Evaluate the board for a given role
  pub fn evaluate(&self, _role: Role) -> i32 {
    // Simplified stub
    0
  }

  // Generate a hash for the board state
  pub fn hash(&self) -> u64 {
    self.zorbist_cache.get_hash()
  }
}
