use std::collections::HashSet;
/// Module handling dictionary management.
use std::fs::File;
use std::io::{BufRead, BufReader};

pub struct ScrabbleDict {}

impl ScrabbleDict {
    pub fn words() -> HashSet<String> {
        let f = match File::open("data/scrabble.txt") {
            Ok(file) => file,
            Err(e) => panic!("Couldn't open dictionary: {:?}", e),
        };
        BufReader::new(f).lines().map(|l| l.unwrap()).collect()
    }

    pub fn words_with_max_length(max_length: usize) -> HashSet<String> {
        Self::words()
            .into_iter()
            .filter(|w| w.len() <= max_length)
            .collect()
    }

    pub fn has_word(word: &String) -> bool {
        Self::words().contains(word)
    }
}
