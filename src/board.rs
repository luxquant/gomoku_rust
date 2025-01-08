use log::info;

use crate::cache::Cache;
use crate::player::Role;
use crate::zobrist_cache::ZobristCache;
use std::collections::HashMap;

const ALL_DIRECTIONS: [[i32; 2]; 4] = [
  [0, 1],  // Горизонтально
  [1, 0],  // Вертикально
  [1, 1],  // Диагональ "\"
  [1, -1], // Диагональ "/"
];

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
}

// Вспомогательная функция, которая возвращает содержимое клетки (px, py) в «терминах паттерна»:
//  - 1, если там «камень текущей роли» (role_val)
//  - 2, если там «противник или стена/вышли-за-грань»
//  - 0, если клетка пуста
fn cell_pattern_value(board_val: i32, role_val: i32) -> i32 {
  if board_val == role_val {
    1
  } else if board_val == 0 {
    0
  } else {
    // либо board_val == 2(стена), либо board_val == -role_val (камень противника)
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

    // let mut shape_cache = HashMap::new();
    // for &r in &[1, -1] {
    //     let mut inner_map = HashMap::new();
    //     for &d in &[0, 1, 2, 3] {
    //         inner_map.insert(d, vec![vec![shape::NONE; size]; size]);
    //     }
    //     shape_cache.insert(r, inner_map);
    // }

    let mut role_scores = HashMap::new();
    for &r in &[Role::White, Role::Black] {
      let mut scores = vec![vec![0; size]; size];
      let center = size / 2;
      scores[center][center] = 1000; // Добавляем больше очков в центр поля
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
        (0, vec![0, 1, 1, 1, 1], 10_000_000),   // FIVE
        (1, vec![1, 0, 1, 1, 1], 10_000_000),   // FIVE
        (2, vec![1, 1, 0, 1, 1], 10_000_000),   // FIVE
        (3, vec![1, 1, 1, 0, 1], 10_000_000),   // FIVE
        (4, vec![1, 1, 1, 1, 0], 10_000_000),   // FIVE
        (1, vec![0, 0, 1, 1, 1, 0], 1_000_000), // OPEN_FOUR
        (2, vec![0, 1, 0, 1, 1, 0], 1_000_000), // OPEN_FOUR
        (3, vec![0, 1, 1, 0, 1, 0], 1_000_000), // OPEN_FOUR
        (4, vec![0, 1, 1, 1, 0, 0], 1_000_000), // OPEN_FOUR
        (1, vec![0, 0, 1, 1, 1, 2], 700_000),   // SEMIOPEN_FOUR
        (2, vec![0, 1, 0, 1, 1, 2], 700_000),   // SEMIOPEN_FOUR
        (3, vec![0, 1, 1, 0, 1, 2], 700_000),   // SEMIOPEN_FOUR
        (4, vec![0, 1, 1, 1, 0, 2], 700_000),   // SEMIOPEN_FOUR
        (4, vec![2, 1, 1, 1, 0, 0], 700_000),   // SEMIOPEN_FOUR
        (3, vec![2, 1, 1, 0, 1, 0], 700_000),   // SEMIOPEN_FOUR
        (2, vec![2, 1, 0, 1, 1, 0], 700_000),   // SEMIOPEN_FOUR
        (1, vec![2, 0, 1, 1, 1, 0], 700_000),   // SEMIOPEN_FOUR
        (1, vec![0, 0, 1, 1, 0], 250_000),      // OPEN_THREE
        (2, vec![0, 1, 0, 1, 0], 250_000),      // OPEN_THREE
        (3, vec![0, 1, 1, 0, 0], 250_000),      // OPEN_THREE
        (1, vec![0, 0, 1, 1, 2], 50_000),       // SEMIOPEN_THREE
        (2, vec![0, 1, 0, 1, 2], 50_000),       // SEMIOPEN_THREE
        (3, vec![0, 1, 1, 0, 2], 50_000),       // SEMIOPEN_THREE
        (3, vec![2, 1, 1, 0, 0], 50_000),       // SEMIOPEN_THREE
        (2, vec![2, 1, 0, 1, 0], 50_000),       // SEMIOPEN_THREE
        (1, vec![2, 0, 1, 1, 0], 50_000),       // SEMIOPEN_THREE
        (1, vec![0, 0, 1, 0], 5_000),           // OPEN_TWO
        (2, vec![0, 1, 0, 0], 5_000),           // OPEN_TWO
        (1, vec![0, 0, 1, 2], 500),             // SEMIOPEN_TWO
        (2, vec![0, 1, 0, 2], 500),             // SEMIOPEN_TWO
        (2, vec![2, 1, 0, 0], 500),             // SEMIOPEN_TWO
        (1, vec![2, 0, 1, 0], 500),             // SEMIOPEN_TWO
        (1, vec![0, 0, 0], 50),                 // OPEN_ONE
        (1, vec![0, 0, 2], 5),                  // SEMIOPEN_ONE
        (1, vec![2, 0, 0], 5),                  // SEMIOPEN_ONE
      ],
      evaluate_cache: Cache::new(0),
      // black_scores: vec![vec![0; size]; size],
      // white_scores: vec![vec![0; size]; size],
      // shape_cache,
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

    // Update Zobrist hash¬
    self.zorbist_cache.toggle_piece(x, y, role.to_int());

    // Обнуление оценок для текущей клетки
    self.role_scores.get_mut(&Role::Black).unwrap()[x][y] = 0;
    self.role_scores.get_mut(&Role::White).unwrap()[x][y] = 0;

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

  /// Пересчитывает (x, y) для обеих ролей и записывает в self.role_scores
  pub fn cacl_score_for_point(&mut self, x: usize, y: usize) {
    // Для каждой роли мы будем аккумулировать score
    for &role in &[Role::White, Role::Black] {
      let role_val = role.to_int(); // +1 или -1
                                    // let opponent_val = -role_val; // -1 или +1

      let mut total_score = 0i32;

      // Проходим по всем шаблонам
      // self.patterns = vec![(activation_index, pattern_vec, cost), ...]
      for &(act_idx, ref pattern_vec, cost) in &self.patterns {
        let pat_len = pattern_vec.len() as i32;

        // Надо проверить 4 направления (гориз, верт, 2 диагонали)
        for &[dx, dy] in &ALL_DIRECTIONS {
          // Идём по всем элементам паттерна и пытаемся совместить их с
          // отрезком на доске длины pat_len, где середина (act_idx) = (x,y).
          // То есть i = 0..(pat_len)
          //
          // px = x + (i - act_idx)*dx
          // py = y + (i - act_idx)*dy

          let mut match_ok = true; // будем проверять, «совпадает» ли весь паттерн
          for i in 0..pat_len {
            let board_x = (x as i32) + (i - act_idx) * dx;
            let board_y = (y as i32) + (i - act_idx) * dy;

            // Если вышли за границы поля - это эквивалент «2» (стена/блок)
            if board_x < 0 || board_x > self.size as i32 - 1 || board_y < 0 || board_y > self.size as i32 - 1 {
              // Считаем что паттерн элемент i == '2'
              if pattern_vec[i as usize] != 2 {
                match_ok = false;
                break;
              }
            } else {
              // Внутри поля => смотрим self.board[ board_x+1, board_y+1 ]
              let real_board_val = self.board[board_x as usize + 1][board_y as usize + 1];
              // Приводим к "паттерн-формату" (0,1,2)
              let cell_val = cell_pattern_value(real_board_val, role_val);
              if cell_val != pattern_vec[i as usize] {
                match_ok = false;
                break;
              }
            }
          } // end for i in 0..pat_len

          if match_ok {
            // Если весь паттерн совпал - добавляем cost
            total_score += cost;
          }
        } // end for directions
      }

      // Записываем total_score в self.role_scores[role][x][y]
      if let Some(matrix) = self.role_scores.get_mut(&role) {
        matrix[x][y] = total_score;
      }
    } // end for role
  }

  // Undo the last move
  pub fn undo(&mut self) -> bool {
    match self.history.pop() {
      // Remove the last move from history
      None => false, // No move to undo
      Some((x, y, _role)) => {
        self.board[x + 1][y + 1] = 0; // Clear the position on the board with adjusted index
        self.zorbist_cache.toggle_piece(x, y, _role.to_int());
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
    info!("AI get_moves {:?}", moves);

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

  /// Генерирует набор «полезных» (ценных) ходов для роли `role`.
  /// - `depth` можно учитывать для более сложной логики.
  /// - `only_three`, `only_four` — если выставлены, оставляем только ходы,
  ///   дающие (или блокирующие) хотя бы «тройку» или «четвёрку».
  pub fn get_moves(&self, role: Role, _depth: i32, only_three: bool, only_four: bool) -> Vec<(usize, usize)> {
    // 1) Собираем все свободные клетки
    let mut candidates: Vec<(usize, usize, i32)> = Vec::new();

    // Предположим, что у нас есть:
    //   - threshold_four = 1_000_000  (OPEN_FOUR cost)
    //   - threshold_three = 250_000   (OPEN_THREE cost)
    let threshold_four = 1_000_000;
    let threshold_three = 250_000;

    // Получаем матрицы очков для текущей роли и для противника
    let my_matrix = &self.role_scores[&role];
    let opp_matrix = &self.role_scores[&role.opponent()];

    for x in 0..self.size {
      for y in 0..self.size {
        // Проверяем, свободна ли клетка
        if self.board[x + 1][y + 1] == 0 {
          // Оцениваем «приоритет» этой клетки
          let my_score = my_matrix[x][y];
          let opp_score = opp_matrix[x][y];
          // Для сортировки возьмём максимум (или сумму, по вкусу)
          let combined_score = my_score.max(opp_score);

          candidates.push((x, y, combined_score));
        }
      }
    }

    // 2) Если у нас выставлен флаг `only_four`, отфильтруем все ходы,
    //    которые не дают (или не блокируют) как минимум «четвёрку».
    if only_four {
      candidates.retain(|&(x, y, _)| {
        let s_my = my_matrix[x][y];
        let s_opp = opp_matrix[x][y];
        // Условно считаем, что "четвёрка" это >= threshold_four
        s_my >= threshold_four || s_opp >= threshold_four
      });
    }

    // 3) Если у нас выставлен флаг `only_three`, отфильтруем все ходы,
    //    которые не дают (или не блокируют) хотя бы «тройку».
    //    (Если одновременно only_four == true, логика может различаться —
    //     но обычно в коде TS эти режимы взаимоисключающие.)
    if only_three {
      candidates.retain(|&(x, y, _)| {
        let s_my = my_matrix[x][y];
        let s_opp = opp_matrix[x][y];
        // Условно считаем, что "тройка" это >= threshold_three
        s_my >= threshold_three || s_opp >= threshold_three
      });
    }

    // 4) Сортируем кандидатов по убыванию (т.е. самые приоритетные — в начале)
    candidates.sort_by_key(|&(_, _, sc)| sc);
    candidates.reverse();

    // // 5) Ограничим размер списка (например, POINTS_LIMIT = 50)
    // let points_limit = 50;
    // candidates.truncate(points_limit);

    // 6) Превращаем (x, y, score) -> (x, y)
    candidates.into_iter().map(|(x, y, _)| (x, y)).collect()
  }

  // Evaluate the board for a given role
  /// Возвращает оценку доски для указанной `role`.
  /// Предполагается, что `self.role_scores[r][x][y]` уже актуальны
  /// (например, после последовательных вызовов `cacl_score_for_point(...)`).
  pub fn evaluate(&mut self, role: Role) -> i32 {
    // Получение хеша доски
    let hash = self.hash();
    // Проверка кэша оценок
    if let Some((prev_role, prev_score)) = self.evaluate_cache.get(&hash) {
      if *prev_role == role {
        return *prev_score;
      }
    }
    // 1) Если уже есть победитель, даём «экстремальное значение»
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
    // Подсчёт очков для чёрных и белых фишек
    for x in 0..self.size {
      for y in 0..self.size {
        black_score += self.role_scores[&Role::Black][x][y];
        white_score += self.role_scores[&Role::White][x][y];
      }
    }
    // Возвращение разницы очков в зависимости от роли
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
