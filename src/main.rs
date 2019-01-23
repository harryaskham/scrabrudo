extern crate rand;
extern crate speculate;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate itertools;
extern crate probability;
#[macro_use]
extern crate approx;
#[macro_use(c)]
extern crate cute;
extern crate bincode;

pub mod bet;
pub mod dict;
pub mod die;
pub mod game;
pub mod hand;
pub mod player;
pub mod testing;
pub mod tile;

use crate::game::*;

use std::collections::HashSet;
use std::env;

fn main() {
    pretty_env_logger::init();

    let args: Vec<String> = env::args().collect();

    info!("Scrabrudo 0.1");
    if args.len() < 3 {
        info!("Please supply mode and number of players");
        return;
    }

    let mode = args[1].parse::<String>().unwrap();
    let num_players = args[2].parse::<usize>().unwrap();
    let mut human_indices = HashSet::new();

    if args.len() >= 4 {
        human_indices.insert(args[3].parse::<usize>().unwrap());
    }

    // TODO: Helper to kill dupe.
    match mode.as_str() {
        "perudo" => {
            let mut game = PerudoGame::new(num_players, 5, human_indices);
            loop {
                game = game.run_turn();
                match game.current_outcome {
                    TurnOutcome::Win => return,
                    _ => continue,
                }
            }
        }
        "scrabrudo" => {
            let mut game = ScrabrudoGame::new(num_players, 5, human_indices);
            loop {
                game = game.run_turn();
                match game.current_outcome {
                    TurnOutcome::Win => return,
                    _ => continue,
                }
            }
        }
        _x => info!("Invalid mode: {}", mode),
    }
}
