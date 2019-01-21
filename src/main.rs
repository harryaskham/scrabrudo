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

pub mod bet;
pub mod game;
pub mod die;
pub mod hand;
pub mod tile;
pub mod player;
pub mod testing;

use crate::game::*;

use std::collections::HashSet;
use std::env;

fn main() {
    pretty_env_logger::init();

    let args: Vec<String> = env::args().collect();

    info!("Perudo 0.1");
    if args.len() < 2 {
        info!("Please supply number of players");
        return;
    }

    let num_players = args[1].parse::<usize>().unwrap();
    let mut human_indices = HashSet::new();

    if args.len() >= 3 {
        human_indices.insert(args[2].parse::<usize>().unwrap());
    }

    let mut game = PerudoGame::new(num_players, human_indices);
    loop {
        game = game.run_turn();
        match game.current_outcome {
            TurnOutcome::Win => return,
            _ => continue,
        }
    }
}
