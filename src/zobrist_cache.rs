use rand::Rng;

/// Structure for storing the Zobrist table and the current hash.
#[derive(Clone, Debug)]
pub struct ZobristCache {
  /// Two-dimensional table: zobrist_table[x][y] = [u64; 2],
  /// where index 0 corresponds to role=1 (black),
  /// and index 1 corresponds to role=-1 (white).
  zobrist_table: Vec<Vec<[u64; 2]>>,
  /// Current sum (XOR) of Zobrist keys.
  hash: u64,
  // /// Size of the game board (gomoku is usually 15, but can be any size).
  // pub size: usize,
}

impl ZobristCache {
  /// Create a new Zobrist table for a board of size `size x size`
  pub fn new(size: usize) -> Self {
    let zobrist_table = Self::initialize_zobrist_table(size);
    ZobristCache {
      zobrist_table,
      hash: 0,
      // size,
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

#[cfg(test)]
mod tests {
  use super::ZobristCache;

  #[test]
  fn test_zobrist_toggle() {
    let size = 5;
    let mut z = ZobristCache::new(size);
    let h0 = z.get_hash();

    // Пусть role_val = +1 (White)
    z.toggle_piece(2, 2, 1);
    let h1 = z.get_hash();
    assert_ne!(h0, h1, "Hash must change after toggling a piece");

    // Ещё раз такой же toggling => хеш вернётся обратно
    z.toggle_piece(2, 2, 1);
    let h2 = z.get_hash();
    assert_eq!(h0, h2, "Hash must revert after toggling same piece again");

    // Проверка на другую клетку/роль
    z.toggle_piece(1, 3, -1);
    let h3 = z.get_hash();
    assert_ne!(h0, h3);
    assert_ne!(h1, h3);
  }

  #[test]
  fn test_zobrist_collisions() {
    // Просто мини-проверка, что при разных позициях
    // хеши с очень малой вероятностью совпадут
    let size = 3;
    let mut z = ZobristCache::new(size);
    let h0 = z.get_hash();
    z.toggle_piece(0, 0, 1);
    let h1 = z.get_hash();
    z.toggle_piece(2, 2, -1);
    let h2 = z.get_hash();

    // Не гарантируем, но ожидаем практически всегда h0 != h1 != h2
    assert_ne!(h0, h1);
    assert_ne!(h0, h2);
    assert_ne!(h1, h2);
  }
}
