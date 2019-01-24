/// Module handling dictionary management.

use std::collections::HashSet;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub struct ScrabbleDict {
    pub words: HashSet<String>,
    pub lookup: HashMap<String, Vec<f64>>,
}

lazy_static! {
    pub static ref SCRABBLE_DICT: ScrabbleDict = ScrabbleDict::new();
}

impl ScrabbleDict {
    pub fn new() -> Self {
        Self {
            words: Self::words(),
            lookup: Self::lookup(),
        }
    }

    /// A set of all words in the dictionary.
    fn words() -> HashSet<String> {
        info!("Loading Scrabble dictionary...");
        let f = match File::open("data/scrabble.txt") {
            Ok(file) => file,
            Err(e) => panic!("Couldn't open dictionary: {:?}", e),
        };
        BufReader::new(f).lines().map(|l| l.unwrap()).collect()
    }

    /// All the words up to a certain length.
    pub fn words_with_max_length(&self, max_length: usize) -> HashSet<String> {
        self.words.clone()
            .into_iter()
            .filter(|w| w.len() <= max_length)
            .collect()
    }

    /// Does the dictionary contain this word?
    pub fn has_word(&self, word: &String) -> bool {
        self.words.contains(word)
    }

    /// Loads the lookup table from sorted char-lists to per-quantity probability lists.
    fn lookup() -> HashMap<String, Vec<f64>> {
        info!("Loading lookup table...");
        // TODO: Predicate this on the number of dice around the table if we have bigger lookups.
        let f = match File::open("data/lookup_5_1000.bin") {
            Ok(file) => file,
            Err(e) => panic!("Couldn't open lookup: {:?}", e),
        };
        bincode::deserialize_from(f).unwrap()
    }
}
