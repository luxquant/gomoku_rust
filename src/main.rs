mod ai;
mod board;
mod cache;
mod cli;
mod game;
mod player;
mod terminal_ui;
mod zobrist_cache;

use crate::cli::{CliArgs, GameModeArg};
use crate::game::{Game, GameMode};
use crate::player::{Player, PlayerType, Role};
use clap::Parser;
use log::info;
use simplelog::*;
use std::fs::File;

fn main() {
  // Initialize the logger to write to a file
  CombinedLogger::init(vec![WriteLogger::new(
    LevelFilter::Info,
    Config::default(),
    File::create("log.txt").unwrap(),
  )])
  .unwrap();

  // 1) Parse command line arguments
  let args = CliArgs::parse();

  info!("Starting game with args: {:?}", args);

  // 2) Convert args.mode to our enum GameMode
  let mode = match args.mode {
    GameModeArg::HumanHuman => GameMode::HumanvHuman,
    GameModeArg::HumanAi => GameMode::AIvHuman,
    GameModeArg::AiAi => GameMode::AIvAI,
  };

  // 3) Define players
  //    For simplicity: player1 is Black, player2 is White.
  //    In real code, you might ask the user who is black and who is white.
  let player1 = match mode {
    GameMode::HumanvHuman => Player {
      player_type: PlayerType::Human,
      role: Role::Black,
      depth: 0,
    },
    GameMode::AIvHuman => Player {
      player_type: PlayerType::Human,
      role: Role::White,
      depth: 0,
    },

    GameMode::AIvAI => Player {
      player_type: PlayerType::AI,
      role: Role::Black,
      depth: 2,
    },
  };

  let player2 = match mode {
    GameMode::HumanvHuman => Player {
      player_type: PlayerType::Human,
      role: Role::White,
      depth: 0,
    },
    GameMode::AIvHuman => Player {
      player_type: PlayerType::AI,
      role: Role::Black,
      depth: 2, // depth, for example
    },
    GameMode::AIvAI => Player {
      player_type: PlayerType::AI,
      role: Role::White,
      depth: 2,
    },
  };

  // 4) Create the game
  let mut game = Game::new(args.size, mode, player1, player2);

  // 5) Run the game
  game.run();
}
