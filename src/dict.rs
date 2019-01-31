use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Mutex;

type Dictionary = HashSet<String>;
type Lookup = HashMap<String, Vec<f64>>;

lazy_static! {
    pub static ref DICT: Mutex<Dictionary> = Mutex::new(Dictionary::new());
    pub static ref LOOKUP: Mutex<Lookup> = Mutex::new(Lookup::new());
}

pub fn init_dict(dict_path: &str) {
    let mut dict = DICT.lock().unwrap();
    *dict = load_dict(dict_path);
}

pub fn init_lookup(lookup_path: &str) {
    let mut lookup = LOOKUP.lock().unwrap();
    *lookup = load_lookup(lookup_path);
}

pub fn has_word(word: &String) -> bool {
    DICT.lock().unwrap().contains(word)
}

/// All the words up to a certain length.
pub fn words_with_max_length(max_length: usize) -> Dictionary {
    DICT.lock().unwrap()
        .clone()
        .into_iter()
        .filter(|w| w.len() <= max_length)
        .collect()
}

/// A set of all words in the dictionary.
fn load_dict(dict_path: &str) -> Dictionary {
    info!("Loading dictionary...");
    let f = match File::open(dict_path) {
        Ok(file) => file,
        Err(e) => panic!("Couldn't open dictionary: {:?}", e),
    };
    BufReader::new(f).lines().map(|l| l.unwrap()).collect()
}

/// Loads the lookup table from sorted char-lists to per-quantity probability lists.
fn load_lookup(lookup_path: &str) -> Lookup {
    info!("Loading lookup table...");
    let f = match File::open(lookup_path) {
        Ok(file) => file,
        Err(e) => panic!("Couldn't open dictionary: {:?}", e),
    };
    bincode::deserialize_from(f).unwrap()
}
