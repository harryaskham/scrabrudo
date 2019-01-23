/// Utility for precomputing the Monte Carlo probabilities for each word in each situation.
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate speculate;
#[macro_use]
extern crate maplit;

pub mod dict;
pub mod testing;

use crate::dict::ScrabbleDict;

use speculate::speculate;
use std::collections::HashMap;
use std::collections::HashSet;

/// Sorts a word by its chars.
fn sort_word(word: &String) -> String {
    let mut chars = word.chars().collect::<Vec<char>>();
    chars.sort_by(|a, b| a.cmp(b));
    chars.iter().collect()
}

/// Generate the word and all its substrings.
/// e.g. HATE, ATE, HTE, HA, HT, HE, AT, AE, TE, H, A, T, E
/// Each word will be sorted to avoid further duplicates:
/// e.g. AEHT, AET, EHT, AH, HT, EH, AT, AE, ET, H, A, T, E
fn all_sorted_substrings(word: &String) -> HashSet<String> {
    if word.len() == 1 {
        return hashset! { sort_word(word) };
    }

    let mut substrings = hashset! { sort_word(word) };

    for i in 0..word.len() {
        let mut word_without = word.clone();
        word_without.remove(i);
        let substrings_without = all_sorted_substrings(&word_without);
        substrings.extend(substrings_without);
    }

    substrings
}

/// Generates all possible valid candidate strings.
/// This is all words plus all non-contiguous substrings of those words.
fn generate_sorted_candidates() -> HashSet<String> {
    HashSet::new()
}

/// Creates a lookup table from word substrings
fn create_lookup(_max_num_tiles: usize) -> HashMap<String, Vec<f64>> {
    HashMap::new()
}

fn main() {
    pretty_env_logger::init();
    info!("TODO");
}

speculate! {
    before {
        testing::set_up();
    }

    describe "substring generation" {
        it "sorts words" {
            assert_eq!("abc", sort_word(&"abc".into()));
            assert_eq!("act", sort_word(&"cat".into()));
            assert_eq!("aeht", sort_word(&"hate".into()));
        }

        it "generates substrings correctly" {
            let expected = hashset! {
                "aht".into(), "et".into(), "aet".into(), "aeht".into(), "e".into(), "ah".into(), "t".into(), "eh".into(), "ht".into(), "ae".into(), "at".into(), "aeh".into(), "h".into(), "eht".into(), "a".into()};
            let actual = all_sorted_substrings(&"hate".into());
            assert_eq!(expected, actual);
        }
    }
}
