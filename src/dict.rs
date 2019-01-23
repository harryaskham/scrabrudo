/// Module handling dictionary management.

use std::fs::File;
use std::io::{BufRead, BufReader};

pub struct ScrabbleDict {}

impl ScrabbleDict {

    pub fn words_with_max_length(max_length: usize) -> Vec<String> {
        let f = match File::open("data/scrabble.txt") {
            Ok(file) => file,
            Err(e) => panic!("Couldn't open dictionary: {:?}", e),
        };
        BufReader::new(f)
            .lines()
            .map(|l| l.unwrap())
            .filter(|l| l.len() <= max_length)
            .collect()
    }
}
