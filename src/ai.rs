use crate::board::Board;
use crate::player::Role;
use crate::player::Role::{Black, White};

pub const MAX: i32 = 100000000;

pub struct AIEngine {
  pub depth: i32,
  // ... other parameters can be stored here
}

impl AIEngine {
  pub fn new(depth: i32) -> Self {
    Self { depth }
  }

  pub fn search_move(&mut self, board: &mut Board, role: Role, depth: i32) -> (i32, Option<(usize, usize)>, Vec<(usize, usize)>) {
    // Start minmax
    self.minmax(board, role, depth)
  }

  fn minmax(&mut self, board: &mut Board, role: Role, depth: i32) -> (i32, Option<(usize, usize)>, Vec<(usize, usize)>) {
    let mut path = Vec::new();
    self.minmax_helper(board, role, depth, 0, &mut path, -MAX, MAX)
  }

  fn minmax_helper(
    &mut self,
    board: &mut Board,
    role: Role,
    depth: i32,
    c_depth: i32,
    path: &mut Vec<(usize, usize)>,
    mut alpha: i32,
    beta: i32,
  ) -> (i32, Option<(usize, usize)>, Vec<(usize, usize)>) {
    // Check if the maximum depth is reached or the game is over
    if c_depth >= depth || board.is_game_over() {
      let score = board.evaluate(role);
      return (score, None, path.clone());
    }

    let mut value = -MAX;
    let mut move_xy = None;
    let mut best_path = path.clone();

    // Get all possible moves
    let points = self.get_valuable_moves_stub(board, role);
    if points.is_empty() {
      let score = board.evaluate(role);
      return (score, None, path.clone());
    }

    // Iterate over all possible moves
    for &(i, j) in &points {
      board.put(i, j, role);
      path.push((i, j));

      // Recursively call minmax_helper for the opponent
      let (mut eval_score, _, child_path) = self.minmax_helper(board, role.opponent(), depth, c_depth + 1, path, -beta, -alpha);

      eval_score = -eval_score;
      path.pop();
      board.undo();

      // Update the best move if a better score is found
      if eval_score > value {
        value = eval_score;
        move_xy = Some((i, j));
        best_path = child_path;
      }

      // Update alpha and check for pruning
      alpha = alpha.max(value);
      if alpha >= beta {
        break;
      }
    }
    (value, move_xy, best_path)
  }

  fn get_valuable_moves_stub(&self, board: &Board, _role: Role) -> Vec<(usize, usize)> {
    let mut moves = Vec::new();
    // Iterate over the board to find empty positions
    for i in 0..board.size {
      for j in 0..board.size {
        if board.board[i][j] == 0 {
          moves.push((i, j));
        }
      }
    }
    moves
  }
}
