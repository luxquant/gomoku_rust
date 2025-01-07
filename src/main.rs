mod cli;
mod game;
mod board;
mod player;
mod ai;
mod terminal_ui;

use clap::Parser;
use crate::cli::{CliArgs, GameModeArg};
use crate::player::{Player, PlayerType, Role};
use crate::game::{Game, GameMode};

fn main() {
    // 1) Parse command line arguments
    let args = CliArgs::parse();

    // 2) Convert args.mode to our enum GameMode
    let mode = match args.mode {
        GameModeArg::HumanHuman => GameMode::HumanvHuman,
        GameModeArg::HumanAi    => GameMode::AIvHuman,
        GameModeArg::AiAi       => GameMode::AIvAI,
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
            player_type: PlayerType::AI,
            role: Role::Black,
            depth: 4, // depth, for example
        },
        GameMode::AIvAI => Player {
            player_type: PlayerType::AI,
            role: Role::Black,
            depth: 4,
        },
    };

    let player2 = match mode {
        GameMode::HumanvHuman => Player {
            player_type: PlayerType::Human,
            role: Role::White,
            depth: 0,
        },
        GameMode::AIvHuman => Player {
            player_type: PlayerType::Human,
            role: Role::White,
            depth: 0,
        },
        GameMode::AIvAI => Player {
            player_type: PlayerType::AI,
            role: Role::White,
            depth: 4,
        },
    };

    // 4) Create the game
    let mut game = Game::new(args.size, mode, player1, player2);

    // 5) Run the game
    game.run();
}
