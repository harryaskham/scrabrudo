use std::collections::HashMap;
/// Module handling dictionary management.
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Mutex;

pub struct ScrabbleDict {
    pub words: HashSet<String>,
    pub lookup: HashMap<String, Vec<f64>>,
}

lazy_static! {
    pub static ref SCRABBLE_DICT: Mutex<ScrabbleDict> = Mutex::new(ScrabbleDict::new());
}

impl ScrabbleDict {
    fn new() -> Self {
        Self {
            words: hashset! {},
            lookup: hashmap! {},
        }
    }

    pub fn init_dict(&mut self, dict_path: &str) {
        self.words = Self::words(dict_path);
    }

    pub fn init_lookup(&mut self, lookup_path: &str) {
        self.lookup = Self::lookup(lookup_path);
    }

    /// A set of all words in the dictionary.
    fn words(dict_path: &str) -> HashSet<String> {
        info!("Loading dictionary...");
        let f = match File::open(dict_path) {
            Ok(file) => file,
            Err(e) => panic!("Couldn't open dictionary: {:?}", e),
        };
        BufReader::new(f).lines().map(|l| l.unwrap()).collect()
    }

    /// All the words up to a certain length.
    pub fn words_with_max_length(&self, max_length: usize) -> HashSet<String> {
        self.words
            .clone()
            .into_iter()
            .filter(|w| w.len() <= max_length)
            .collect()
    }

    /// Does the dictionary contain this word?
    pub fn has_word(&self, word: &String) -> bool {
        self.words.contains(word)
    }

    /// Loads the lookup table from sorted char-lists to per-quantity probability lists.
    fn lookup(lookup_path: &str) -> HashMap<String, Vec<f64>> {
        info!("Loading lookup table...");
        let f = match File::open(lookup_path) {
            Ok(file) => file,
            Err(e) => panic!("Couldn't open dictionary: {:?}", e),
        };
        bincode::deserialize_from(f).unwrap()
    }
}
