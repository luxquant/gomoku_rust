use log::info;
use tracing::instrument;

use crate::cache::Cache;
use crate::patterns::GOMOKU_PATTERNS;
use crate::player::Role;
use crate::zobrist_cache::ZobristCache;
use std::collections::HashMap;

const DIRECTIONS: usize = 4;

const ALL_DIRECTIONS: [[i32; 2]; 4] = [
  [1, 0],  // Horizontal
  [0, 1],  // Vertical
  [1, 1],  // Diagonal "\"
  [-1, 1], // Diagonal "/"
];

/// For convenience, we will make enum templates or IDs (we could store the cost directly).
/// But we will use the existing self.patterns.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeId {
  None = 0,
  // Here you can list all (Five, OpenFour, etc.),
  // but in our example we store only the index from patterns
  Pattern(usize),
}

/// We store for each cell (x, y), for each role (White/Black),
/// for each of the 4 directions:
///   - shape_id (index of the pattern that "matched" exactly when activating (x,y))
///   - dirty: bool (whether it needs to be recalculated)
#[derive(Clone, Debug)]
pub struct ShapeCache {
  /// shape_cache[role][dir][x][y] = (shape_id, cost)
  /// role can be mapped to 0..1 (0=Black, 1=White)
  pub data: Vec<Vec<Vec<Vec<(ShapeId, i32)>>>>,
  pub dirty: Vec<Vec<Vec<Vec<bool>>>>,
}

impl ShapeCache {
  /// Create ShapeCache for a board of size `size`.
  pub fn new(size: usize) -> Self {
    let data = vec![vec![vec![vec![(ShapeId::None, 0); size]; size]; DIRECTIONS]; 2];
    let dirty = vec![vec![vec![vec![false; size]; size]; DIRECTIONS]; 2];

    ShapeCache { data, dirty }
  }

  /// Mark the cell (x,y) for a specific role and all directions as "dirty" —
  /// to recalculate the shape on the next request.
  pub fn mark_dirty(&mut self, role: Role, x: usize, y: usize) {
    let r_idx = role_index(role);
    // mark all 4 directions
    for dir in 0..DIRECTIONS {
      self.dirty[r_idx][dir][x][y] = true;
    }
  }

  /// Mark the "neighborhood" of the cell (x,y) as dirty (similar to recalc_scores logic),
  /// so that the patterns are found again on the next request.
  pub fn mark_neighbors_dirty(&mut self, role: Role, x: usize, y: usize, size: usize) {
    let dirs = [[0, 1], [1, 0], [1, 1], [1, -1]];
    self.mark_dirty(role, x, y);

    for [dx, dy] in dirs {
      for sign in [-1, 1] {
        for step in 1..=5 {
          let nx = x as isize + (sign * step) * dx as isize;
          let ny = y as isize + (sign * step) * dy as isize;
          if nx < 0 || ny < 0 || nx >= size as isize || ny >= size as isize {
            break;
          }
          self.mark_dirty(role, nx as usize, ny as usize);
        }
      }
    }
  }
}

/// Utility to map Role::Black -> 0, Role::White -> 1
fn role_index(r: Role) -> usize {
  match r {
    Role::Black => 0,
    Role::White => 1,
  }
}

#[derive(Clone, Debug)]
struct ValuableMovesCacheEntry {
  role: Role,
  moves: Vec<(usize, usize)>,
  depth: i32,
  only_three: bool,
  only_four: bool,
}

#[derive(Clone, Debug)]
pub struct Board {
  pub size: usize,
  pub board: Vec<Vec<i32>>,               // 0=empty, +1=white, -1=black
  pub history: Vec<(usize, usize, Role)>, // History of moves
  zorbist_cache: ZobristCache,
  winner_cache: Cache<u64, i32>,
  gameover_cache: Cache<u64, bool>,
  evaluate_cache: Cache<u64, (Role, i32)>,
  valuable_moves_cache: Cache<u64, ValuableMovesCacheEntry>,

  role_scores: HashMap<Role, Vec<Vec<i32>>>,
  patterns: &'static [(i32, &'static [i32], i32)],
  shape_cache: ShapeCache,
}

// Helper function that returns the content of the cell (px, py) in "pattern terms":
//  - 1, if there is a "stone of the current role" (role_val)
//  - 2, if there is an "opponent or wall/out-of-bounds"
//  - 0, if the cell is empty
fn cell_pattern_value(board_val: i32, role_val: i32) -> i32 {
  if board_val == role_val {
    1
  } else if board_val == 0 {
    0
  } else {
    // either board_val == 2 (wall), or board_val == -role_val (opponent's stone)
    2
  }
}

impl Board {
  // Create a new board with the given size
  pub fn new(size: usize) -> Self {
    let size_with_wall = size + 2;
    // Create a 2D vector of size_with_wall x size_with_wall, filled with values of 2
    let mut b: Vec<Vec<i32>> = vec![vec![2; size_with_wall]; size_with_wall];
    for i in 1..=size {
      for j in 1..=size {
        b[i][j] = 0;
      }
    }

    let mut role_scores = HashMap::new();
    for &r in &[Role::White, Role::Black] {
      let mut scores = vec![vec![0; size]; size];
      let center = size / 2;
      scores[center][center] = 1000; // Add more points to the center of the board
      role_scores.insert(r, scores);
    }

    Self {
      size,
      board: b,
      history: Vec::new(),                    // Initialize an empty history
      zorbist_cache: ZobristCache::new(size), // Initialize Zobrist cache for the board size
      winner_cache: Cache::new(0),            // Initialize winner cache
      gameover_cache: Cache::new(0),          // Initialize gameover cache
      valuable_moves_cache: Cache::new(0),    // Initialize valuable moves cache
      role_scores,
      patterns: GOMOKU_PATTERNS,
      evaluate_cache: Cache::new(0),
      shape_cache: ShapeCache::new(size),
    }
  }

  // Place a stone on the board
  pub fn put(&mut self, x: usize, y: usize, role: Role) -> bool {
    if x >= self.size || y >= self.size {
      // Check if the position is out of bounds
      return false;
    }
    if self.board[x + 1][y + 1] != 0 {
      // Check if the position is already occupied
      return false;
    }
    self.board[x + 1][y + 1] = role.to_int(); // Place the stone
    self.history.push((x, y, role)); // Record the move in history with adjusted index

    // Update Zobrist hash
    self.zorbist_cache.toggle_piece(x, y, role.to_int());

    // Reset scores for the current cell
    self.role_scores.get_mut(&Role::Black).unwrap()[x][y] = 0;
    self.role_scores.get_mut(&Role::White).unwrap()[x][y] = 0;

    // Mark shape_cache.roleScores as "dirty"
    self.shape_cache.mark_neighbors_dirty(role, x, y, self.size);
    self.shape_cache.mark_neighbors_dirty(role.opponent(), x, y, self.size);

    self.recalc_scores(x, y);

    true
  }

  #[instrument]
  fn recalc_scores(&mut self, x: usize, y: usize) {
    if self.board[x + 1][y + 1] == 0 {
      self.cacl_score_for_point(x, y);
    }

    for &[dx, dy] in &ALL_DIRECTIONS {
      for &sign in &[1, -1] {
        for step in 1..=4 {
          let nx = (x as i32 + sign * step * dx) as i32;
          let ny = (y as i32 + sign * step * dy) as i32;
          if nx < 0 || nx >= self.size as i32 || ny < 0 || ny >= self.size as i32 {
            break;
          }
          let bx = (nx + 1) as usize;
          let by = (ny + 1) as usize;
          if self.board[bx][by] == 0 {
            self.cacl_score_for_point(bx - 1, by - 1);
          }
        }
      }
    }
  }

  /// Example of a fully updated cacl_score_for_point that uses shape_cache.
  pub fn cacl_score_for_point(&mut self, x: usize, y: usize) {
    // Reset score=0 for (x,y) for both roles — then we will sum up
    *self.value_mut(Role::Black, x, y) = 0;
    *self.value_mut(Role::White, x, y) = 0;

    // For each role — sum up 4 directions:
    for &role in &[Role::Black, Role::White] {
      // check/update shape_cache if dirty
      self.update_cache_if_needed(role, x, y);

      // After update_cache_if_needed shape_cache.data[r_idx][x][y][dir]
      // should already contain (shape_id, cost).
      // We just need to "traverse" them and sum up the cost.
      let r_idx = role_index(role);
      let mut total_score = 0;

      for dir in 0..DIRECTIONS {
        let (_, cost) = self.shape_cache.data[r_idx][dir][x][y];
        total_score += cost;
      }

      // ====== ADDING DEFENSIVE LOGIC ======
      // Check how dangerous the opponent's patterns are at this cell (x,y).
      // If the opponent can get a big combination here (say, >= 1_000_000),
      // it is important to "block" (from the perspective of the current role).
      let opp = role.opponent();
      self.update_cache_if_needed(opp, x, y); // recalculate if dirty
      let opp_idx = role_index(opp);
      let mut opp_threat_score = 0;
      for dir in 0..DIRECTIONS {
        let (_, cost) = self.shape_cache.data[opp_idx][dir][x][y];
        opp_threat_score += cost;
      }

      // If the opponent at this cell potentially makes an OPEN_FOUR (~5_000_000) or higher —
      // this is a very dangerous point, we need to defend.
      if opp_threat_score >= 2_000_000 {
        total_score += 1_500_000; // very dangerous opponent move
      } else if opp_threat_score >= 1_000_000 {
        total_score += 1_000_000; // just dangerous
      } else if opp_threat_score >= 300_000 {
        total_score += 600_000; // semi-open 4
      }

      // Write total_score
      *self.value_mut(role, x, y) = total_score;
    }
  }

  /// If shape_cache for (role, x, y, dir) is "dirty", recalculate the pattern in this direction,
  /// write (shapeId, cost). And so for all dir=0..3.
  fn update_cache_if_needed(&mut self, role: Role, x: usize, y: usize) {
    let r_idx = role_index(role);

    for dir in 0..DIRECTIONS {
      if self.shape_cache.dirty[r_idx][dir][x][y] {
        // Нужно пересчитать
        let (sh_id, total_cost) = self.find_best_pattern_in_dir(role, x, y, dir);

        // В shape_cache.data[r_idx][dir][x][y] мы храним (ShapeId, i32).
        // Запишем найденный shape_id + суммарный cost
        self.shape_cache.data[r_idx][dir][x][y] = (sh_id, total_cost);

        // Сбрасываем dirty
        self.shape_cache.dirty[r_idx][dir][x][y] = false;
      }
    }
  }

  /// Returns `(ShapeId, total_cost)`, where `ShapeId` is the ID of the pattern with the maximum cost,
  /// and `total_cost` is the sum of all matched patterns.
  /// Thus, if the point (x,y) creates multiple threats, they will be summed.
  #[instrument]
  fn find_best_pattern_in_dir(&self, role: Role, x: usize, y: usize, dir: usize) -> (ShapeId, i32) {
    let role_val = role.to_int();
    let (dx, dy) = match dir {
      0 => (1, 0),  // horizontal direction
      1 => (0, 1),  // vertical direction
      2 => (1, 1),  // diagonal direction "\"
      3 => (-1, 1), // diagonal direction "/"
      _ => (1, 0),  // default fallback
    };

    let mut best_cost = 0; // cost of the most expensive pattern
    let mut best_shape = ShapeId::None;
    let mut sum_cost = 0; // sum of costs of all matched patterns

    for (i_pattern, &(act_idx, ref pattern_vec, cost)) in self.patterns.iter().enumerate() {
      // Let's apply a small heuristic to skip
      // very cheap patterns if the game is already advanced
      if self.history.len() > 2 && cost < 200 {
        continue;
      }

      // Check for pattern match
      if self.check_pattern(role_val, x, y, dx, dy, act_idx, pattern_vec) {
        // If matched, add cost to sum_cost
        sum_cost += cost;
        // Compare if this is the most expensive pattern
        if cost > best_cost {
          best_cost = cost;
          best_shape = ShapeId::Pattern(i_pattern);
        }
      }
    }

    // Return (ShapeId of the most expensive, sum)
    (best_shape, sum_cost)
  }

  /// Check if pattern_vec matches when "activating" (x,y),
  /// in the direction (dx,dy), if act_idx is the "activation point".
  fn check_pattern(
    &self,
    role_val: i32,
    x: usize,
    y: usize,
    dx: i32,
    dy: i32,
    act_idx: i32,
    pattern_vec: &'static [i32],
  ) -> bool {
    let pat_len = pattern_vec.len() as i32;

    for i in 0..pat_len {
      let board_x = x as i32 + (i - act_idx) * dx;
      let board_y = y as i32 + (i - act_idx) * dy;
      // Check for out of bounds
      if board_x < 0 || board_x >= self.size as i32 || board_y < 0 || board_y >= self.size as i32 {
        // Compare pattern_vec[i] with 2
        if pattern_vec[i as usize] != 2 {
          return false;
        }
      } else {
        // inside the board
        let real_val = self.board[board_x as usize + 1][board_y as usize + 1];
        let cell_val = cell_pattern_value(real_val, role_val);
        if cell_val != pattern_vec[i as usize] {
          return false;
        }
      }
    }
    true
  }

  /// Get role score at position (x, y) for logging purposes
  pub fn get_role_score(&self, role: Role, x: usize, y: usize) -> i32 {
    self.role_scores.get(&role).map(|scores| scores[x][y]).unwrap_or(0)
  }

  /// Find all critical threats from opponent that must be defended
  /// Returns positions where opponent would get a strong position
  pub fn find_critical_threats(&mut self, role: Role) -> Vec<(usize, usize, i32)> {
    let opponent = role.opponent();
    let mut threats = Vec::new();

    // Get baseline evaluation before any moves
    let baseline_eval = self.evaluate(opponent);

    // Check all empty positions by simulating opponent moves
    for x in 0..self.size {
      for y in 0..self.size {
        if self.board[x + 1][y + 1] == 0 {
          // Simulate opponent move
          self.put(x, y, opponent);

          // Check if this creates a winning position
          if self.check_five(x, y, opponent) {
            self.undo();
            threats.push((x, y, 10_000_000)); // FIVE - must block immediately!
            continue;
          }

          // Evaluate position change after opponent's move
          let after_eval = self.evaluate(opponent);
          let eval_gain = after_eval - baseline_eval;

          // Undo the move
          self.undo();

          // Critical threats based on evaluation gain:
          // 3M+ = Very strong position (likely four or double threat)
          // 1M+ = Strong position (open/semi-open four)
          // 500K+ = Moderate threat (strong three)
          if eval_gain >= 500_000 {
            threats.push((x, y, eval_gain));
          }
        }
      }
    }

    // Sort by threat level (highest first)
    threats.sort_by(|a, b| b.2.cmp(&a.2));
    threats
  }

  /// Check if there's a five in a row at position (x, y) for the given role
  fn check_five(&self, x: usize, y: usize, role: Role) -> bool {
    let role_val = role.to_int();
    let bx = x + 1;
    let by = y + 1;

    // Check all 4 directions
    for &[dx, dy] in &ALL_DIRECTIONS {
      let mut count = 1; // count the stone we just placed

      // Count in positive direction
      for step in 1..5 {
        let nx = bx as i32 + step * dx;
        let ny = by as i32 + step * dy;
        if nx < 0 || nx >= (self.size + 2) as i32 || ny < 0 || ny >= (self.size + 2) as i32 {
          break;
        }
        if self.board[nx as usize][ny as usize] != role_val {
          break;
        }
        count += 1;
      }

      // Count in negative direction
      for step in 1..5 {
        let nx = bx as i32 - step * dx;
        let ny = by as i32 - step * dy;
        if nx < 0 || nx >= (self.size + 2) as i32 || ny < 0 || ny >= (self.size + 2) as i32 {
          break;
        }
        if self.board[nx as usize][ny as usize] != role_val {
          break;
        }
        count += 1;
      }

      if count >= 5 {
        return true;
      }
    }

    false
  }

  /// Utility: get a reference to `role_scores[role][x][y]`.
  fn value_mut(&mut self, role: Role, x: usize, y: usize) -> &mut i32 {
    self
      .role_scores
      .get_mut(&role)
      .unwrap()
      .get_mut(x)
      .unwrap()
      .get_mut(y)
      .unwrap()
  }

  // Undo the last move
  pub fn undo(&mut self) -> bool {
    match self.history.pop() {
      // Remove the last move from history
      None => false, // No move to undo
      Some((x, y, _role)) => {
        self.board[x + 1][y + 1] = 0; // Clear the position on the board with adjusted index
        self.zorbist_cache.toggle_piece(x, y, _role.to_int());

        // +++ IMPORTANT +++
        // mark shape_cache around (x,y) as dirty
        self.shape_cache.mark_neighbors_dirty(_role, x, y, self.size);
        self.shape_cache.mark_neighbors_dirty(_role.opponent(), x, y, self.size);

        self.recalc_scores(x, y);
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

  #[instrument]
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
    let moves = self.get_moves(role, depth, only_three, only_four);

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

  /// Generates a set of "valuable" moves for the role `role`.
  /// - `depth` can be considered for more complex logic.
  /// - `only_three`, `only_four` — if set, leave only moves that give (or block) at least a "three" or "four".
  pub fn get_moves(&self, role: Role, _depth: i32, only_three: bool, only_four: bool) -> Vec<(usize, usize)> {
    // 1) Collect all free cells
    let mut candidates: Vec<(usize, usize, i32)> = Vec::new();

    // Assume we have:
    //   - threshold_four = 1_000_000  (OPEN_FOUR cost)
    //   - threshold_three = 250_000   (OPEN_THREE cost)
    let threshold_four = 1_000_000;
    let threshold_three = 250_000;

    // Get score matrices for the current role and the opponent
    let my_matrix = &self.role_scores[&role];
    let opp_matrix = &self.role_scores[&role.opponent()];

    for x in 0..self.size {
      for y in 0..self.size {
        // Check if the cell is free
        if self.board[x + 1][y + 1] == 0 {
          // Evaluate the "priority" of this cell
          let my_score = my_matrix[x][y];
          let opp_score = opp_matrix[x][y];
          // For sorting, take the maximum (or sum, as you like)
          let combined_score = my_score.max(opp_score);

          candidates.push((x, y, combined_score));
        }
      }
    }

    // 2) If we have the `only_four` flag set, filter out all moves
    //    that do not give (or block) at least a "four".
    if only_four {
      candidates.retain(|&(x, y, _)| {
        let s_my = my_matrix[x][y];
        let s_opp = opp_matrix[x][y];
        // Conditionally consider that "four" is >= threshold_four
        s_my >= threshold_four || s_opp >= threshold_four
      });
    }

    // 3) If we have the `only_three` flag set, filter out all moves
    //    that do not give (or block) at least a "three".
    //    (If only_four == true at the same time, the logic may differ —
    //     but usually in TS code these modes are mutually exclusive.)
    if only_three {
      candidates.retain(|&(x, y, _)| {
        let s_my = my_matrix[x][y];
        let s_opp = opp_matrix[x][y];
        // Conditionally consider that "three" is >= threshold_three
        s_my >= threshold_three || s_opp >= threshold_three
      });
    }

    // 4) Sort candidates in descending order (i.e., the most priority ones are at the beginning)
    candidates.sort_by_key(|&(_, _, sc)| sc);
    candidates.reverse();

    // 6) Convert (x, y, score) -> (x, y)
    candidates.into_iter().map(|(x, y, _)| (x, y)).collect()
  }

  // Evaluate the board for a given role
  /// Returns the board evaluation for the specified `role`.
  /// It is assumed that `self.role_scores[r][x][y]` are already up-to-date
  /// (e.g., after consecutive calls to `cacl_score_for_point(...)`).
  #[instrument]
  pub fn evaluate(&mut self, role: Role) -> i32 {
    // Get the board hash
    let hash = self.hash();
    // Check the evaluation cache
    if let Some((prev_role, prev_score)) = self.evaluate_cache.get(&hash) {
      if *prev_role == role {
        return *prev_score;
      }
    }
    // 1) If there is already a winner, give an "extreme value"
    let winner = self.get_winner();
    if winner == role.to_int() {
      return 10_000_000;
    } else if winner == -role.to_int() {
      return -10_000_000;
    }

    let score = self.evaluate_internal(role);
    self.evaluate_cache.put(hash, (role, score));
    score
  }

  fn evaluate_internal(&self, role: Role) -> i32 {
    let mut black_score = 0;
    let mut white_score = 0;
    // Count points for black and white stones
    for x in 0..self.size {
      for y in 0..self.size {
        black_score += self.role_scores[&Role::Black][x][y];
        white_score += self.role_scores[&Role::White][x][y];
      }
    }
    // Return the difference in points depending on the role
    if role == Role::Black {
      black_score - white_score
    } else {
      white_score - black_score
    }
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

  // Implement the display method for debugging the board
  pub fn display(&self) {
    for y in 1..=self.size {
      for x in 1..=self.size {
        let cell = self.board[x][y];
        let symbol = match cell {
          1 => "W",  // White stone
          -1 => "B", // Black stone
          0 => ".",  // Empty cell
          _ => " ",  // Wall or out-of-bounds (should not happen inside the board)
        };
        print!("{} ", symbol);
      }
      println!();
    }
  }
}

// src/board.rs

#[cfg(test)]
mod tests_board {
  use super::*;
  use crate::player::Role;

  #[test]
  fn test_board_put_undo() {
    let mut board = Board::new(5);
    assert_eq!(board.history.len(), 0);

    // Try to place White at (2,2)
    let ok = board.put(2, 2, Role::White);
    assert!(ok);
    assert_eq!(board.board[3][3], 1); // white=+1
    assert_eq!(board.history.len(), 1);

    // Undo
    let undone = board.undo();
    assert!(undone);
    assert_eq!(board.board[3][3], 0);
    assert_eq!(board.history.len(), 0);
  }

  #[test]
  fn test_put_out_of_bounds() {
    let mut board = Board::new(5);
    // Valid indices: 0..4
    // Place out of bounds
    let ok1 = board.put(5, 2, Role::Black);
    let ok2 = board.put(4, 10, Role::Black);
    assert!(!ok1);
    assert!(!ok2);
    assert_eq!(board.history.len(), 0);
  }

  #[test]
  fn test_put_on_occupied() {
    let mut board = Board::new(5);
    board.put(1, 1, Role::Black);
    // Place again on the same spot
    let ok = board.put(1, 1, Role::White);
    assert!(!ok, "Should not allow placing on occupied cell");
    assert_eq!(board.history.len(), 1);
  }
}

#[cfg(test)]
mod tests_pattern {
  use super::*;
  use crate::player::Role;

  #[test]
  fn test_check_pattern_simple() {
    let mut board = Board::new(5);

    // Place white=+1 at (1,1), (1,2), (1,3)
    board.put(1, 1, Role::White);
    board.put(1, 2, Role::White);
    board.put(1, 3, Role::White);

    // Manually check the pattern:
    //  dir=0 => dx=0,dy=1 (horizontal)
    let role_val = Role::White.to_int();
    let found = board.check_pattern(
      role_val,
      1,
      2, // x=1,y=2 — activation point (middle)
      0,
      1,
      1,                // act_idx=1
      &[1, 1, 1, 0, 0], // Using an existing pattern from GOMOKU_PATTERNS
    );
    assert!(found, "Should detect the pattern [1,1,1] with activation in the middle");
  }

  #[test]
  fn test_find_best_pattern_in_dir() {
    let mut b = Board::new(10);
    // // Define 2 patterns (cost=500, cost=1000)
    // b.patterns.push((1, vec![1, 1, 1], 500));
    // b.patterns.push((1, vec![1, 1, 1, 1], 1000)); // "FOUR" simplified

    // Place 4 white stones horizontally (1,1),(1,2),(1,3),(1,4)
    b.put(1, 1, Role::White);
    b.put(1, 2, Role::White);
    b.put(1, 3, Role::White);
    b.put(1, 4, Role::White);

    // b.display();

    // Check find_best_pattern_in_dir for (1,2) with dir=0(hor)
    let (sh_id, cost) = b.find_best_pattern_in_dir(Role::White, 1, 0, 1);
    println!("sh_id: {:?}, cost: {:?}", sh_id, cost);
    match sh_id {
      ShapeId::Pattern(idx) => {
        let pat = &b.patterns[idx];
        assert_eq!(pat.2, 4_000_000);
      }
      _ => panic!("Pattern not found"),
    }
    assert_eq!(cost, 4_000_000);

    // If at (2,2) dir=0 => no white stones => cost=0
    let (sh2, cost2) = b.find_best_pattern_in_dir(Role::White, 2, 2, 1);
    match sh2 {
      ShapeId::Pattern(idx) => {
        let pat = &b.patterns[idx];
        assert_eq!(pat.2, 10);
      }
      _ => panic!("Pattern not found"),
    }
    assert_eq!(cost2, 10);
  }
}

#[cfg(test)]
mod tests_scoring {
  use super::*;
  use crate::player::Role;

  #[test]
  fn test_cacl_score_for_point_defensive() {
    // Check that with dangerous patterns from the opponent,
    // we get additional points for this point
    let mut brd = Board::new(10);

    // Suppose there are already 3 consecutive Black stones, and cost>=1_000_000 => OpenFour
    // For simplicity, artificially change board.patterns cost.
    // (or add)
    // Suppose the cost of "OpenFour" = 2_000_000,
    //    => then opp_threat_score >=2_000_000 => add +800000

    // Place black= -1 at (1,1),(1,2),(1,3) => leave (1,4) free
    brd.put(1, 1, Role::Black);
    brd.put(1, 2, Role::Black);
    brd.put(1, 3, Role::Black);

    // Suppose now White is considering the cell (1,4)
    // (simulate cacl_score_for_point)
    brd.cacl_score_for_point(1, 4);

    let wsc = brd.role_scores[&Role::White][1][4];
    let bsc = brd.role_scores[&Role::Black][1][4];

    println!("wsc: {:?}, bsc: {:?}", wsc, bsc);

    // If (1,4) allows Black to "close" an "OpenFour" =>
    //   opp_threat_score=2_000_000 => => +800_000
    // Accordingly, wsc should be >= 800_000
    // bsc can also be significant, but in this case
    //   Black to place there? (1,4)?
    //   However, bsc can also be large, but not less than 800k (depends on patterns).
    assert!(wsc >= 800_000, "White sees a big threat from Black => adds defense bonus");
    assert!(
      bsc >= 2_000_000,
      "Black also sees the same location as a finishing move => big score"
    );
  }
}

#[cfg(test)]
mod tests_winner {
  use super::*;
  use crate::player::Role;

  #[test]
  fn test_no_winner_initial() {
    let mut b = Board::new(5);
    let w = b.get_winner();
    assert_eq!(w, 0);
    assert!(!b.is_game_over());
  }

  #[test]
  fn test_winner_black_horizontal() {
    let mut b = Board::new(5);
    // Place 5 consecutive stones horizontally
    // (2,2),(3,2),(4,2),(5,2),(6,2) - but the actual field size=5 => "walls"
    // Correct, we have inside ( x+1, y+1 ),
    // so logical coordinates 0..4
    //  => (0,2),(1,2),(2,2),(3,2),(4,2)
    b.put(0, 2, Role::Black);
    b.put(1, 2, Role::Black);
    b.put(2, 2, Role::Black);
    b.put(3, 2, Role::Black);
    b.put(4, 2, Role::Black);

    let w = b.get_winner();
    assert_eq!(w, -1, "Black's role_val=-1 => means black wins");
    assert!(b.is_game_over());
  }

  #[test]
  fn test_winner_white_diagonal() {
    let mut b = Board::new(5);
    // White stones diagonally (0,0),(1,1),(2,2),(3,3),(4,4)
    for i in 0..5 {
      b.put(i, i, Role::White);
    }
    let w = b.get_winner();
    assert_eq!(w, 1, "White=+1");
    assert!(b.is_game_over());
  }
}

#[cfg(test)]
mod tests_moves {
  use super::*;
  use crate::player::Role;

  #[test]
  fn test_get_moves_basic() {
    let mut b = Board::new(5);
    // Fill with 2 Black stones, 2 White stones
    b.put(0, 0, Role::Black);
    b.put(4, 4, Role::White);
    b.put(1, 1, Role::White);
    // Check that get_moves returns the remaining free cells (5*5 -3=22)
    let moves = b.get_moves(Role::Black, 1, false, false);
    assert_eq!(moves.len(), 22);
  }

  #[test]
  fn test_get_moves_filter_four() {
    let mut b = Board::new(5);
    // Suppose some position where (2,2) gives OpenFour>=1_000_000
    // Simplify => manually set scores:
    b.role_scores.get_mut(&Role::Black).unwrap()[2][2] = 1_500_000;
    // "only_four=true"
    let moves = b.get_moves(Role::Black, 2, false, true);
    // should contain (2,2), as score>=1_000_000 => "FOUR"
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0], (2, 2));
  }

  #[test]
  fn test_get_valuable_moves_cached() {
    let mut b = Board::new(5);
    // First call to fill valuable_moves_cache
    let mv1 = b.get_valuable_moves(Role::White, 2, false, false);
    // Second call => should take from cache
    let mv2 = b.get_valuable_moves(Role::White, 2, false, false);
    assert_eq!(mv1, mv2);
    // Check that something was returned
    assert!(!mv1.is_empty());
  }
}
