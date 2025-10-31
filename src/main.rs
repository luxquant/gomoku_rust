mod ai;
mod board;
mod cache;
mod cli;
mod game;
mod game_logger;
mod patterns;
mod player;
mod terminal_ui;
mod zobrist_cache;

use crate::cli::{CliArgs, FirstPlayerArg, GameModeArg};
use crate::game::{Game, GameMode};
use crate::player::{Player, PlayerType, Role};
use clap::Parser;
use log::info;
// use simplelog::*;
// use std::fs::File;

fn main() {
  // Configure tracing to write to a file
  // let subscriber = FmtSubscriber::builder()
  // .with_max_level(tracing::Level::INFO)
  // .with_writer(std::fs::File::create("profiling.log").unwrap()) // Specify the output file
  // .finish();

  // tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

  // Initialize the logger to write to a file
  // CombinedLogger::init(vec![WriteLogger::new(
  //   LevelFilter::Info,
  //   Config::default(),
  //   File::create("log.txt").unwrap(),
  // )])
  // .unwrap();

  // 1) Parse command line arguments
  let args = CliArgs::parse();

  info!("Starting game with args: {:?}", args);

  // 2) Convert args.mode to our enum GameMode
  let mode = match args.mode {
    GameModeArg::HumanHuman => GameMode::HumanvHuman,
    GameModeArg::HumanAi => GameMode::AIvHuman,
    GameModeArg::AiAi => GameMode::AIvAI,
  };

  // 3) Define players based on the game mode
  let (player1, player2) = match mode {
    GameMode::HumanvHuman => (
      Player {
        player_type: PlayerType::Human,
        role: Role::Black,
        depth: 0,
      },
      Player {
        player_type: PlayerType::Human,
        role: Role::White,
        depth: 0,
      },
    ),
    GameMode::AIvHuman => match args.first_player {
      FirstPlayerArg::Human => (
        Player {
          player_type: PlayerType::Human,
          role: Role::Black,
          depth: 0,
        },
        Player {
          player_type: PlayerType::AI,
          role: Role::White,
          depth: args.depth,
        },
      ),
      FirstPlayerArg::AI => (
        Player {
          player_type: PlayerType::AI,
          role: Role::Black,
          depth: args.depth,
        },
        Player {
          player_type: PlayerType::Human,
          role: Role::White,
          depth: 0,
        },
      ),
    },
    GameMode::AIvAI => (
      Player {
        player_type: PlayerType::AI,
        role: Role::Black,
        depth: args.depth,
      },
      Player {
        player_type: PlayerType::AI,
        role: Role::White,
        depth: args.depth,
      },
    ),
  };

  // 4) Create the game instance
  let mut game = Game::new(args.size, mode, player1, player2);

  // 5) Run the game loop
  if args.log {
    game.run_with_logging();
  } else {
    game.run();
  }
}
