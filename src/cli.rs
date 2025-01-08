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
}
