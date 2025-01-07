use crate::cache::Cache;
use crate::player::Role;
use crate::zobrist_cache::ZobristCache;

#[derive(Clone)]
struct ValuableMovesCacheEntry {
  role: Role,
  moves: Vec<(usize, usize)>,
  depth: i32,
  only_three: bool,
  only_four: bool,
}

#[derive(Clone)]
pub struct Board {
  pub size: usize,
  pub board: Vec<Vec<i32>>,               // 0=empty, +1=white, -1=black
  pub history: Vec<(usize, usize, Role)>, // History of moves
  pub zorbist_cache: ZobristCache,
  pub winner_cache: Cache<u64, i32>,
  pub gameover_cache: Cache<u64, bool>,
  pub valuable_moves_cache: Cache<u64, ValuableMovesCacheEntry>,
}

impl Board {
  // Create a new board with the given size
  pub fn new(size: usize) -> Self {
    let size_with_wall = size + 2;
    // Create a 2D vector of size_with_wall x size_with_wall, filled with values of 2
    let mut b = vec![vec![2; size_with_wall]; size_with_wall];
    for i in 1..=size {
      for j in 1..=size {
        b[i][j] = 0;
      }
    }
    Self {
      size,
      board: b,
      history: Vec::new(),                    // Initialize an empty history
      zorbist_cache: ZobristCache::new(size), // Initialize Zobrist cache for the board size
      winner_cache: Cache::new(0),            // Initialize winner cache
      gameover_cache: Cache::new(0),          // Initialize gameover cache
      valuable_moves_cache: Cache::new(0),    // Initialize valuable moves cache
    }
  }

  // Place a stone on the board
  pub fn put(&mut self, i: usize, j: usize, role: Role) -> bool {
    let i = i + 1; // Adjust index for the wall
    let j = j + 1; // Adjust index for the wall
    if i >= self.size + 1 || j >= self.size + 1 {
      // Check if the position is out of bounds
      return false;
    }
    if self.board[i][j] != 0 {
      // Check if the position is already occupied
      return false;
    }
    self.board[i][j] = role.to_int(); // Place the stone
    self.history.push((i - 1, j - 1, role)); // Record the move in history with adjusted index

    // Update Zobrist hash
    self.zorbist_cache.toggle_piece(i - 1, j - 1, role.to_int());

    true
  }

  // Undo the last move
  pub fn undo(&mut self) -> bool {
    match self.history.pop() {
      // Remove the last move from history
      None => false, // No move to undo
      Some((i, j, _role)) => {
        self.board[i + 1][j + 1] = 0; // Clear the position on the board with adjusted index
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

    for i in 1..=self.size {
      for j in 1..=self.size {
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
    for i in 1..=self.size {
      for j in 1..=self.size {
        let cell = self.board[i][j];
        if cell == 0 {
          continue;
        }
        for &(dx, dy) in &directions {
          let mut count = 0;
          while i as isize + dx * count >= 1
            && i as isize + dx * count <= self.size as isize
            && j as isize + dy * count >= 1
            && j as isize + dy * count <= self.size as isize
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

  pub fn get_valuable_moves(&mut self, role: Role, depth: i32, only_three: bool, only_four: bool) -> Vec<(usize, usize)> {
    // Get the board hash
    let hash = self.hash();
    // Check the valuable moves cache
    if let Some(prev) = self.valuable_moves_cache.get(&hash) {
      if prev.role == role && prev.depth == depth && prev.only_three == only_three && prev.only_four == only_four {
        return prev.moves.clone();
      }
    }

    // Get possible moves
    let mut moves = self.get_moves(role, depth, only_three, only_four);

    // Add a random choice among free cells in the center
    if !only_three && !only_four {
      let center = (self.size / 2) as isize;
      let center_cells = vec![
        (center - 1, center - 1),
        (center - 1, center),
        (center - 1, center + 1),
        (center, center - 1),
        (center, center),
        (center, center + 1),
        (center + 1, center - 1),
        (center + 1, center),
        (center + 1, center + 1),
      ];
      let free_center_cells: Vec<(usize, usize)> = center_cells
        .into_iter()
        .filter(|&(x, y)| self.board[(x + 1) as usize][(y + 1) as usize] == 0)
        .map(|(x, y)| ((x + 1) as usize, (y + 1) as usize))
        .collect();
      if !free_center_cells.is_empty() {
        let random_index = rand::random::<usize>() % free_center_cells.len();
        moves.push(free_center_cells[random_index]);
      }
    }

    // Save valuable moves to cache
    self.valuable_moves_cache.put(
      hash,
      ValuableMovesCacheEntry {
        role,
        moves: moves.clone(),
        depth,
        only_three,
        only_four,
      },
    );
    moves
  }

  pub fn get_moves(&self, role: Role, depth: i32, only_three: bool, only_four: bool) -> Vec<(usize, usize)> {
    // Simplified stub
    vec![]
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

  pub fn reverse(&self) -> Board {
    let mut new_board = Board::new(self.size);
    for &(x, y, role) in &self.history {
      new_board.put(x, y, role.opponent());
    }
    new_board
  }
}
