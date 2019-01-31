use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Mutex;

type Dictionary = HashSet<String>;

lazy_static! {
    static ref DICT: Mutex<Option<Dictionary>> = Mutex::new(None);
    static ref LOOKUP: Mutex<Option<sled::Db>> = Mutex::new(None);
}

pub fn init_dict(dict_path: &str) {
    let mut dict = DICT.lock().unwrap();
    *dict = Some(load_dict(dict_path));
}

pub fn init_lookup(lookup_path: &str) {
    let mut lookup = LOOKUP.lock().unwrap();
    *lookup = Some(sled::Db::start_default(lookup_path).unwrap());
}

pub fn dict() -> Dictionary {
    DICT.lock().unwrap().clone().unwrap()
}

fn lookup() -> sled::Db {
    LOOKUP.lock().unwrap().clone().unwrap()
}

pub fn has_word(word: &String) -> bool {
    dict().contains(word)
}

/// All the words up to a certain length.
pub fn words_with_max_length(max_length: usize) -> Dictionary {
    dict()
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

/// Does the sled DB contain the word?
pub fn lookup_has(s: &str) -> bool {
   lookup().contains_key(s).unwrap()
}

/// Pull the encoded list out of the sled DB.
/// None if we don't have probs for this.
pub fn lookup_probs(s: &str) -> Option<Vec<f64>> {
    let encoded_probs = match lookup().get(s).unwrap() {
        Some(ps) => ps,
        None => return None
    };
    bincode::deserialize(&(encoded_probs.to_owned())[..]).unwrap()
}

/// How many keys?
pub fn lookup_len() -> usize {
    lookup().keys(vec![]).count()
}
