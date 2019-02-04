use sstable::{Options, SSIterator, Table};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Mutex;

type Dictionary = HashSet<String>;

lazy_static! {
    static ref DICT: Mutex<Option<Dictionary>> = Mutex::new(None);
    static ref LOOKUP: Mutex<Option<String>> = Mutex::new(None);
}

pub fn init_dict(dict_path: &str) {
    let mut dict = DICT.lock().unwrap();
    *dict = Some(load_dict(dict_path));
}

pub fn init_lookup(lookup_path: &str) {
    let mut lookup = LOOKUP.lock().unwrap();
    *lookup = Some(lookup_path.into());
}

pub fn dict() -> Dictionary {
    DICT.lock().unwrap().clone().unwrap()
}

fn lookup() -> Table {
    let lookup_path = LOOKUP.lock().unwrap().clone().unwrap();
    Table::new_from_file(Options::default(), Path::new(&lookup_path)).unwrap()
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

/// Does the lookup contain the word?
pub fn lookup_has(s: &str) -> bool {
    match lookup().get(s.as_bytes()).unwrap() {
        Some(_) => true,
        None => false,
    }
}

/// Pull the encoded list out of the storage.
/// None if we don't have probs for this.
pub fn lookup_probs(s: &str) -> Option<Vec<f64>> {
    let encoded_probs = match lookup().get(s.as_bytes()).unwrap() {
        Some(ps) => ps,
        None => return None,
    };
    Some(bincode::deserialize(&encoded_probs).unwrap())
}

/// How many keys?
pub fn lookup_len() -> usize {
    let mut len = 0;
    let mut iter = lookup().iter();
    loop {
        match iter.next() {
            Some(_) => len += 1,
            None => return len,
        }
    }
}
