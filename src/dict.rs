use std::collections::HashMap;
/// Module handling dictionary management.
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub struct ScrabbleDict {
    pub words: HashSet<String>,
    pub lookup: HashMap<String, Vec<f64>>,
}

lazy_static! {
    // TODO: Change this for actual gameplay to use a Mutex and a mutable lazy init with the
    // flag values.
    pub static ref SCRABBLE_DICT: ScrabbleDict = ScrabbleDict::new(
        "data/scrabble.txt",
        "data/lookup_5_1000.bin"
    );
}

impl ScrabbleDict {
    pub fn new(dict_path: &str, lookup_path: &str) -> Self {
        Self {
            words: Self::words(dict_path),
            lookup: Self::lookup(lookup_path),
        }
    }

    /// A set of all words in the dictionary.
    fn words(dict_path: &str) -> HashSet<String> {
        info!("Loading Scrabble dictionary...");
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
            Err(e) => {
                warn!("Couldn't open lookup: {:?}", e);
                return hashmap!{};
            }
        };
        bincode::deserialize_from(f).unwrap()
    }
}
