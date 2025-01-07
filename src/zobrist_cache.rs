use rand::Rng;

/// Structure for storing the Zobrist table and the current hash.
#[derive(Clone)]
pub struct ZobristCache {
  /// Two-dimensional table: zobrist_table[x][y] = [u64; 2],
  /// where index 0 corresponds to role=1 (black),
  /// and index 1 corresponds to role=-1 (white).
  zobrist_table: Vec<Vec<[u64; 2]>>,
  /// Current sum (XOR) of Zobrist keys.
  hash: u64,
  /// Size of the game board (gomoku is usually 15, but can be any size).
  pub size: usize,
}

impl ZobristCache {
  /// Create a new Zobrist table for a board of size `size x size`
  pub fn new(size: usize) -> Self {
    let zobrist_table = Self::initialize_zobrist_table(size);
    ZobristCache {
      zobrist_table,
      hash: 0,
      size,
    }
  }

  /// Initialize the Zobrist table for each cell [x][y] and for each role (1 / -1).
  fn initialize_zobrist_table(size: usize) -> Vec<Vec<[u64; 2]>> {
    let mut table = vec![vec![[0u64; 2]; size]; size];
    let mut rng = rand::thread_rng();

    for x in 0..size {
      for y in 0..size {
        // We have two "roles": role=1 (black) and role=-1 (white).
        // To simplify, we place them in indices 0 and 1 respectively.
        table[x][y][0] = rng.gen::<u64>(); // for role=1
        table[x][y][1] = rng.gen::<u64>(); // for role=-1
      }
    }
    table
  }

  /// Toggle (XOR) the hash value for the stone `role` at cell (x,y).
  /// The `role` parameter is expected to be `1` (black) or `-1` (white).
  pub fn toggle_piece(&mut self, x: usize, y: usize, role: i32) {
    // Convert role (1/-1) to index 0/1.
    let role_index = if role == 1 { 0 } else { 1 };
    self.hash ^= self.zobrist_table[x][y][role_index];
  }

  /// Returns the current Zobrist hash value.
  pub fn get_hash(&self) -> u64 {
    self.hash
  }
}
