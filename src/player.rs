#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerType {
  Human,
  AI,
}

// Role of the stone
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
  Black, // -1
  White, // +1
}

impl Role {
  // Get the opponent's role
  pub fn opponent(&self) -> Role {
    match self {
      Role::Black => Role::White,
      Role::White => Role::Black,
    }
  }

  // Convert role to integer
  pub fn to_int(&self) -> i32 {
    match self {
      Role::Black => -1,
      Role::White => 1,
    }
  }
}

pub struct Player {
  pub player_type: PlayerType,
  pub role: Role,

  // Parameters for AI, search depth
  pub depth: i32,
  // Other settings for AI can be stored
  // (e.g., heuristics, cache, ...)
}
