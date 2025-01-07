use crate::board::Board;
use crate::cache::Cache;
use crate::player::Role;
use crate::shapes::shape;

pub const MAX: i32 = 100000000;

/// Structure to account for cache statistics
#[derive(Debug, Default)]
pub struct CacheHits {
  pub search: i32,
  pub total: i32,
  pub hit: i32,
}

#[derive(Clone)]
pub struct CacheEntry {
  pub depth: i32,
  pub value: i32,
  pub role: Role,
  pub move_xy: Option<(usize, usize)>,
  pub path: Vec<(usize, usize)>,
  pub only_three: bool,
  pub only_four: bool,
}

pub struct AIEngine {
  pub depth: i32,
  pub cache_hits: CacheHits,

  cache: Cache<u64, CacheEntry>,

  // Equivalent to `private onlyThreeThreshold: number;` in TS
  only_three_threshold: i32,
}

impl AIEngine {
  /// Constructor: takes the search depth `depth`.
  pub fn new(depth: i32) -> Self {
    Self {
      depth,
      cache_hits: CacheHits::default(),
      cache: Cache::new(0),
      only_three_threshold: 6,
    }
  }

  /// Analog of the TS method `analyze(...)`
  #[allow(clippy::too_many_arguments)]
  fn analyze(
    &mut self,
    only_three: bool,
    only_four: bool,
    board: &mut Board,
    role: Role,
    depth: i32,
    cdepth: i32,
    path: &mut Vec<(usize, usize)>,
    mut alpha: i32,
    beta: i32,
  ) -> (i32, Option<(usize, usize)>, Vec<(usize, usize)>) {
    self.cache_hits.search += 1;

    // 1) Base exit conditions
    if cdepth >= depth || board.is_game_over() {
      let score = board.evaluate(role);
      return (score, None, path.clone());
    }

    // 2) Cache check
    let hash_val = board.hash();
    if let Some(prev) = self.cache.get(&hash_val) {
      // Assume that `prev` returns as some structure/enum
      // which stores:
      //   - prev.value: i32
      //   - prev.role: i32
      //   - prev.depth: i32
      //   - prev.move_xy: Option<(usize, usize)>
      //   - prev.only_three: bool
      //   - prev.only_four: bool
      //   - prev.path: Vec<(usize, usize)>
      if prev.role == role {
        let depth_left = depth - cdepth;
        if (prev.value.abs() >= shape::FIVE || prev.depth >= depth_left)
          && prev.only_three == only_three
          && prev.only_four == only_four
        {
          self.cache_hits.hit += 1;
          let new_path = {
            // path + prev.path
            let mut p = path.clone();
            p.extend_from_slice(&prev.path);
            p
          };
          return (prev.value, prev.move_xy, new_path);
        }
      }
    }

    // 3) Initialize variables
    let mut value = -MAX;
    let mut best_move: Option<(usize, usize)> = None;
    let mut best_path = path.clone();
    let mut best_depth = best_path.len() as i32;

    // 4) Generate "valuable" moves
    let points = board.get_valuable_moves(role, cdepth, only_three || cdepth > self.only_three_threshold, only_four);
    if points.is_empty() {
      let score = board.evaluate(role);
      return (score, None, path.clone());
    }

    // 5) Depth loop
    'depthLoop: for d in (cdepth + 1)..=depth {
      // 6) Iterate over all "valuable" moves
      for p in &points {
        let (px, py) = *p;
        board.put(px, py, role);

        // Add move to path
        path.push((px, py));

        let (mut eval_score, _eval_move, mut eval_path) = self.analyze(
          only_three,
          only_four,
          board,
          role.opponent(),
          d,
          cdepth + 1,
          path,
          -beta,
          -alpha,
        );

        // 7) Undo
        board.undo();
        path.pop();

        // Return to own role
        eval_score = -eval_score;

        // 8) Compare with maximum
        if eval_score >= shape::FIVE || d == depth {
          if eval_score > value || (eval_score <= -shape::FIVE && value <= -shape::FIVE && eval_path.len() as i32 > best_depth) {
            value = eval_score;
            best_path = eval_path.clone();
            best_depth = best_path.len() as i32;
            best_move = Some((px, py));
          }
        }

        // 9) Alpha-beta
        alpha = alpha.max(value);
        if alpha >= shape::FIVE {
          break 'depthLoop;
        }
        if alpha >= beta {
          break;
        }
      }
    }

    // 10) Save to cache (if needed)
    //    (In TS: if ((cDepth < self.onlyThreeThreshold || onlyThree || onlyFour) && ...)
    let depth_left = depth - cdepth;
    let do_put = (cdepth < self.only_three_threshold as i32) || only_three || only_four;
    if do_put {
      // form structure for storage
      // path for cache: cut best_path, but usually "bestPath.slice(cDepth)"
      let sliced_path = {
        let mut p = Vec::new();
        if best_path.len() as i32 >= cdepth {
          // cdepth is index
          let idx = cdepth as usize;
          p.extend_from_slice(&best_path[idx..]);
        }
        p
      };

      self.cache.put(
        hash_val,
        CacheEntry {
          depth: depth_left,
          value,
          role,
          move_xy: best_move,
          path: sliced_path,
          only_three,
          only_four,
        },
      );
      self.cache_hits.total += 1;
    }

    (value, best_move, best_path)
  }

  pub fn make_move(&mut self, board: &mut Board, role: Role) -> (i32, Option<(usize, usize)>, Vec<(usize, usize)>) {
    let vct_depth = self.depth + self.depth * 2;

    // 1) First try to analyze with (onlyThree=true, onlyFour=false)
    //    similar to "let [value, move, path] = this.analyze(true, false, ...)"
    let mut path_buf = vec![];
    let (mut value, mut mv, mut path) = self.analyze(true, false, board, role, vct_depth, 0, &mut path_buf, -MAX, MAX);

    // If the score >= SCORES.FIVE => direct return
    if value >= shape::FIVE {
      return (value, mv, path);
    }

    // 2) Otherwise (onlyThree=false, onlyFour=false)
    let mut path_buf2 = vec![];
    let (value2, mv2, path2) = self.analyze(false, false, board, role, self.depth, 0, &mut path_buf2, -MAX, MAX);

    value = value2;
    mv = mv2;
    path = path2;

    if mv.is_none() {
      return (value, None, path);
    }

    // 3) Make a move on the board to check further
    let (mx, my) = mv.unwrap();
    board.put(mx, my, role);

    // 4) Look at "value2, move2, path2" with (onlyThree=true, board.reverse(), vctDepth)
    let rev_board = board.reverse();
    let mut path_buf3 = vec![];
    let (value_rev, move_rev, path_rev) = self.analyze(
      true,
      false,
      &mut rev_board.clone(),
      role,
      vct_depth,
      0,
      &mut path_buf3,
      -MAX,
      MAX,
    );

    board.undo(); // Undo

    if value < shape::FIVE && value_rev == shape::FIVE && path_rev.len() > path.len() {
      // Additional check:
      let mut path_buf4 = vec![];
      let (value_rev2, move_rev2, path_rev2) = self.analyze(
        true,
        false,
        &mut rev_board.clone(),
        role,
        vct_depth,
        0,
        &mut path_buf4,
        -MAX,
        MAX,
      );

      if path_rev.len() <= path_rev2.len() {
        // Return move_rev
        return (value, move_rev, path_rev);
      }
    }

    (value, mv, path)
  }
}
