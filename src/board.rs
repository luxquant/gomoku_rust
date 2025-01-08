use crate::cache::Cache;
use crate::player::Role;
use crate::zobrist_cache::ZobristCache;
use std::collections::HashMap;

const DIRECTIONS: usize = 4;

const ALL_DIRECTIONS: [[i32; 2]; 4] = [
  [0, 1],  // Horizontal
  [1, 0],  // Vertical
  [1, 1],  // Diagonal "\"
  [1, -1], // Diagonal "/"
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
#[derive(Clone)]
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
  zorbist_cache: ZobristCache,
  winner_cache: Cache<u64, i32>,
  gameover_cache: Cache<u64, bool>,
  evaluate_cache: Cache<u64, (Role, i32)>,
  valuable_moves_cache: Cache<u64, ValuableMovesCacheEntry>,

  role_scores: HashMap<Role, Vec<Vec<i32>>>,
  patterns: Vec<(i32, Vec<i32>, i32)>,
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
      patterns: vec![
        (0, vec![0, 1, 1, 1, 1], 10_000_000),      // FIVE
        (1, vec![1, 0, 1, 1, 1], 10_000_000),      // FIVE
        (2, vec![1, 1, 0, 1, 1], 10_000_000),      // FIVE
        (3, vec![1, 1, 1, 0, 1], 10_000_000),      // FIVE
        (4, vec![1, 1, 1, 1, 0], 10_000_000),      // FIVE
        (1, vec![0, 0, 1, 1, 1, 0], 5_000_000),    // OPEN_FOUR
        (4, vec![0, 1, 1, 1, 0, 0], 5_000_000),    // OPEN_FOUR
        (2, vec![0, 1, 0, 1, 1, 0], 5_000_000),    // OPEN_FOUR
        (3, vec![0, 1, 1, 0, 1, 0], 5_000_000),    // OPEN_FOUR
        (5, vec![2, 0, 1, 1, 1, 0, 0], 5_000_000), // SEMIOPEN_FOUR
        (1, vec![0, 0, 1, 1, 1, 0, 2], 5_000_000), // SEMIOPEN_FOUR
        (1, vec![0, 0, 1, 1, 1, 2], 300_000),      // SEMIOPEN_FOUR
        (2, vec![0, 1, 0, 1, 1, 2], 300_000),      // SEMIOPEN_FOUR
        (3, vec![0, 1, 1, 0, 1, 2], 300_000),      // SEMIOPEN_FOUR
        (4, vec![0, 1, 1, 1, 0, 2], 300_000),      // SEMIOPEN_FOUR
        (4, vec![2, 1, 1, 1, 0, 0], 300_000),      // SEMIOPEN_FOUR
        (3, vec![2, 1, 1, 0, 1, 0], 300_000),      // SEMIOPEN_FOUR
        (2, vec![2, 1, 0, 1, 1, 0], 300_000),      // SEMIOPEN_FOUR
        (1, vec![0, 0, 1, 1, 0], 250_000),         // OPEN_THREE
        (2, vec![0, 1, 0, 1, 0], 250_000),         // OPEN_THREE
        (3, vec![0, 1, 1, 0, 0], 250_000),         // OPEN_THREE
        (1, vec![0, 0, 1, 1, 2], 20_000),          // SEMIOPEN_THREE
        (2, vec![0, 1, 0, 1, 2], 20_000),          // SEMIOPEN_THREE
        (3, vec![0, 1, 1, 0, 2], 20_000),          // SEMIOPEN_THREE
        (3, vec![2, 1, 1, 0, 0], 20_000),          // SEMIOPEN_THREE
        (2, vec![2, 1, 0, 1, 0], 20_000),          // SEMIOPEN_THREE
        (1, vec![2, 0, 1, 1, 0], 20_000),          // SEMIOPEN_THREE
        (1, vec![0, 0, 1, 0], 3_000),              // OPEN_TWO
        (2, vec![0, 1, 0, 0], 3_000),              // OPEN_TWO
        (1, vec![0, 0, 1, 2], 200),                // SEMIOPEN_TWO
        (2, vec![0, 1, 0, 2], 200),                // SEMIOPEN_TWO
        (2, vec![2, 1, 0, 0], 200),                // SEMIOPEN_TWO
        (1, vec![2, 0, 1, 0], 200),                // SEMIOPEN_TWO
        (1, vec![0, 0, 0], 30),                    // OPEN_ONE
        (1, vec![0, 0, 2], 1),                     // SEMIOPEN_ONE
        (1, vec![2, 0, 0], 1),                     // SEMIOPEN_ONE
      ],
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

  fn recalc_scores(&mut self, x: usize, y: usize) {
    if self.board[x + 1][y + 1] == 0 {
      self.cacl_score_for_point(x, y);
    }

    for &[dx, dy] in &ALL_DIRECTIONS {
      for &sign in &[1, -1] {
        for step in 1..=5 {
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
        // Need to recalculate
        let (sh_id, cost) = self.find_best_pattern_in_dir(role, x, y, dir);

        // Update in cache
        self.shape_cache.data[r_idx][dir][x][y] = (sh_id, cost);
        // Remove dirty flag
        self.shape_cache.dirty[r_idx][dir][x][y] = false;
      }
    }
  }

  /// Here we do roughly the same logic as in the old cacl_score_for_point,
  /// but only for one direction (dir).
  /// Return (ShapeId, cost) that was found "best".
  fn find_best_pattern_in_dir(&self, role: Role, x: usize, y: usize, dir: usize) -> (ShapeId, i32) {
    let role_val = role.to_int();
    let (dx, dy) = match dir {
      0 => (0, 1),  // horizontal
      1 => (1, 0),  // vertical
      2 => (1, 1),  // diagonal "\"
      3 => (1, -1), // diagonal "/"
      _ => (0, 1),  // fallback
    };

    let mut best_cost = 0;
    let mut best_shape = ShapeId::None;

    // self.patterns: [(act_idx, pattern_vec, cost), ...]
    for (i_pattern, &(act_idx, ref pattern_vec, cost)) in self.patterns.iter().enumerate() {
      let pat_len = pattern_vec.len() as i32;

      // Check for match
      if self.check_pattern(role_val, x, y, dx, dy, act_idx, pattern_vec) {
        // if it matches — compare cost with best_cost
        if cost > best_cost {
          best_cost = cost;
          best_shape = ShapeId::Pattern(i_pattern);
        }
      }
    }
    (best_shape, best_cost)
  }

  /// Check if pattern_vec matches when "activating" (x,y),
  /// in the direction (dx,dy), if act_idx is the "activation point".
  fn check_pattern(&self, role_val: i32, x: usize, y: usize, dx: i32, dy: i32, act_idx: i32, pattern_vec: &Vec<i32>) -> bool {
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
        .map(|(x, y)| (x as usize, y as usize))
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
}
