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
#[macro_use]
extern crate lazy_static;
extern crate rayon;
extern crate clap;
#[macro_use]
extern crate maplit;

pub mod bet;
pub mod dict;
pub mod die;
pub mod game;
pub mod hand;
pub mod player;
pub mod testing;
pub mod tile;

use crate::dict::*;
use crate::game::*;

use std::collections::HashSet;
use clap::App;

fn main() {
    pretty_env_logger::init();

    let matches = App::new("Scrabrudo")
       .version("0.1")
       .about("A mixture of Scrabble and Perudo")
       .author("Harry Askham")
       .args_from_usage("-m, --mode=[MODE] 'perudo or scrabrudo'
                        -n, --num_players=[NUM_PLAYERS] 'the number of players'
                        -h, --human_index=[HUMAN_INDEX] 'which, if any, is the human'
                        -d, --dictionary_path=[DICTIONARY] 'the path to the .txt dict to use'
                        -l, --lookup_path=[LOOKUP] 'the path to the .bin lookup to write'")
       .get_matches(); 

    let mode = matches.value_of("mode").unwrap_or("scrabrudo");
    let num_players: usize = matches.value_of("num_players").unwrap_or("2").parse::<usize>().unwrap();
    let mut human_indices: HashSet<usize> = hashset!{};
    match matches.value_of("human_index") {
        Some(x) => { human_indices.insert(x.parse::<usize>().unwrap()); }
        None => ()
    };

    let dict_path = matches.value_of("dictionary_path").unwrap();
    let lookup_path = matches.value_of("lookup_path").unwrap();

    SCRABBLE_DICT.lock().unwrap().init_dict(dict_path);
    SCRABBLE_DICT.lock().unwrap().init_lookup(lookup_path);

    // TODO: Helper to kill dupe.
    match mode {
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
            info!(
                "Loaded lookup: {} items",
                SCRABBLE_DICT.lock().unwrap().lookup.len()
            );
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
