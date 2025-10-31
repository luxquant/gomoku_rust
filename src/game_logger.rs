use crate::board::Board;
use crate::player::Role;
use std::fs::File;
use std::io::Write;

pub struct GameLogger {
  file: File,
  move_number: i32,
}

impl GameLogger {
  pub fn new(filename: &str) -> std::io::Result<Self> {
    let file = File::create(filename)?;
    Ok(Self { file, move_number: 0 })
  }

  pub fn log_move_start(&mut self, role: Role, round: i32) -> std::io::Result<()> {
    self.move_number = round;
    writeln!(
      self.file,
      "\n{}\nMove #{} - Player: {:?} ({})\n{}",
      "=".repeat(80),
      round,
      role,
      if role == Role::Black { "X" } else { "O" },
      "=".repeat(80)
    )
  }

  pub fn log_board_state(&mut self, board: &Board) -> std::io::Result<()> {
    writeln!(self.file, "\nCurrent board state:")?;
    writeln!(
      self.file,
      "   {}",
      (0..board.size).map(|i| format!("{:2}", i)).collect::<Vec<_>>().join(" ")
    )?;

    for y in 0..board.size {
      write!(self.file, "{:2} ", y)?;
      for x in 0..board.size {
        let val = board.board[x + 1][y + 1];
        let ch = match val {
          0 => ".",
          1 => "O",
          -1 => "X",
          _ => "?",
        };
        write!(self.file, " {} ", ch)?;
      }
      writeln!(self.file)?;
    }
    writeln!(self.file)
  }

  pub fn log_candidates(&mut self, candidates: &[(usize, usize)], _role: Role) -> std::io::Result<()> {
    writeln!(self.file, "\nCandidate moves ({}): ", candidates.len())?;
    for (i, &(x, y)) in candidates.iter().enumerate().take(10) {
      if i > 0 && i % 5 == 0 {
        writeln!(self.file)?;
      }
      write!(self.file, "  ({:2},{:2})", x, y)?;
    }
    if candidates.len() > 10 {
      writeln!(self.file, "\n  ... and {} more", candidates.len() - 10)?;
    }
    writeln!(self.file)
  }

  pub fn log_analysis_result(
    &mut self,
    stage: &str,
    value: i32,
    best_move: Option<(usize, usize)>,
    path: &[(usize, usize)],
    depth: i32,
  ) -> std::io::Result<()> {
    writeln!(self.file, "\n{} analysis:", stage)?;
    writeln!(self.file, "  Depth: {}", depth)?;
    writeln!(self.file, "  Evaluation: {}", value)?;
    writeln!(self.file, "  Best move: {:?}", best_move)?;

    if !path.is_empty() {
      writeln!(self.file, "  Predicted path ({} moves):", path.len())?;
      for (i, &(x, y)) in path.iter().enumerate().take(10) {
        if i > 0 && i % 5 == 0 {
          writeln!(self.file)?;
        }
        write!(self.file, "    ({:2},{:2})", x, y)?;
      }
      if path.len() > 10 {
        writeln!(self.file, "\n    ... and {} more moves", path.len() - 10)?;
      }
      writeln!(self.file)?;
    }
    Ok(())
  }

  pub fn log_patterns(&mut self, x: usize, y: usize, role: Role, board: &Board) -> std::io::Result<()> {
    writeln!(self.file, "\nPattern analysis for position ({}, {}):", x, y)?;

    // Get scores for this position from board evaluation
    let my_score = board.get_role_score(role, x, y);
    let opp_score = board.get_role_score(role.opponent(), x, y);

    writeln!(self.file, "  My position score: {}", my_score)?;
    writeln!(self.file, "  Opponent position score: {}", opp_score)?;

    // Decode threat level based on opponent score
    if opp_score >= 4_000_000 {
      writeln!(self.file, "  THREAT LEVEL: CRITICAL - Opponent has FIVE!")?;
    } else if opp_score >= 2_000_000 {
      writeln!(self.file, "  THREAT LEVEL: HIGH - Opponent has open FOUR!")?;
    } else if opp_score >= 1_000_000 {
      writeln!(self.file, "  THREAT LEVEL: MEDIUM - Opponent has semi-open FOUR")?;
    } else if opp_score >= 250_000 {
      writeln!(self.file, "  THREAT LEVEL: LOW - Opponent has THREE pattern")?;
    }

    // Decode opportunity level based on my score
    if my_score >= 4_000_000 {
      writeln!(self.file, "  OPPORTUNITY: WINNING - This creates FIVE!")?;
    } else if my_score >= 2_000_000 {
      writeln!(self.file, "  OPPORTUNITY: EXCELLENT - This creates open FOUR!")?;
    } else if my_score >= 1_000_000 {
      writeln!(self.file, "  OPPORTUNITY: GOOD - This creates semi-open FOUR")?;
    } else if my_score >= 250_000 {
      writeln!(self.file, "  OPPORTUNITY: MODERATE - This creates THREE pattern")?;
    }

    Ok(())
  }

  pub fn log_final_decision(&mut self, chosen_move: Option<(usize, usize)>, value: i32, reason: &str) -> std::io::Result<()> {
    writeln!(self.file, "\n*** FINAL DECISION ***")?;
    writeln!(self.file, "  Chosen move: {:?}", chosen_move)?;
    writeln!(self.file, "  Final evaluation: {}", value)?;
    writeln!(self.file, "  Reason: {}", reason)?;
    writeln!(self.file, "\n")?;
    self.file.flush()
  }

  pub fn log_cache_stats(&mut self, hit: i32, total: i32, search: i32) -> std::io::Result<()> {
    if total > 0 {
      let hit_rate = (hit as f64 / total as f64) * 100.0;
      writeln!(self.file, "\nCache statistics:")?;
      writeln!(self.file, "  Cache hits: {} / {} ({:.1}%)", hit, total, hit_rate)?;
      writeln!(self.file, "  Total searches: {}", search)?;
    }
    Ok(())
  }

  pub fn log_game_end(&mut self, winner: i32, total_moves: i32) -> std::io::Result<()> {
    writeln!(self.file, "\n\n{}", "=".repeat(80))?;
    writeln!(self.file, "GAME OVER")?;
    writeln!(self.file, "{}", "=".repeat(80))?;

    match winner {
      0 => writeln!(self.file, "Result: DRAW")?,
      1 => writeln!(self.file, "Result: WHITE (O) WINS!")?,
      -1 => writeln!(self.file, "Result: BLACK (X) WINS!")?,
      _ => writeln!(self.file, "Result: UNKNOWN")?,
    }

    writeln!(self.file, "Total moves: {}", total_moves)?;
    writeln!(self.file, "{}\n", "=".repeat(80))?;
    self.file.flush()
  }
}
