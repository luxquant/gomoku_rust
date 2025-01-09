use clap::{Parser, ValueEnum};

/// Game mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum GameModeArg {
  /// Human vs Human
  HumanHuman,
  /// Human vs AI
  HumanAi,
  /// AI vs AI
  AiAi,
}

/// First player
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FirstPlayerArg {
  Human,
  AI,
}

/// Gomoku
#[derive(Parser, Debug)]
#[command(name = "gomoku_rust", version = "0.1.0")]
pub struct CliArgs {
  /// Game mode
  #[arg(long, value_enum, default_value_t=GameModeArg::HumanHuman)]
  pub mode: GameModeArg,

  /// Field size
  #[arg(long, default_value_t = 15)]
  pub size: usize,

  /// AI depth
  #[arg(long, default_value_t = 3)]
  pub depth: i32,

  /// First player in Human vs AI mode
  #[arg(long, value_enum, default_value_t=FirstPlayerArg::Human)]
  pub first_player: FirstPlayerArg,
}
