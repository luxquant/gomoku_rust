#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerType {
  Human, // Represents a human player
  AI,    // Represents an AI player
}

// Role of the stone
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
  Black, // Role for black stones, represented by -1
  White, // Role for white stones, represented by +1
}

impl Role {
  // Get the opponent's role
  pub fn opponent(&self) -> Role {
    match self {
      Role::Black => Role::White, // If current role is Black, opponent is White
      Role::White => Role::Black, // If current role is White, opponent is Black
    }
  }

  // Convert role to integer
  pub fn to_int(&self) -> i32 {
    match self {
      Role::Black => -1, // Black role corresponds to -1
      Role::White => 1,  // White role corresponds to +1
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub struct Player {
  pub player_type: PlayerType, // Type of player (Human or AI)
  pub role: Role,              // Role of the player (Black or White)

  // Parameters for AI, search depth
  pub depth: i32, // Depth of search for AI
                  // Other settings for AI can be stored
                  // (e.g., heuristics, cache, ...)
}
