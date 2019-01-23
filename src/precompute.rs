/// Utility for precomputing the Monte Carlo probabilities for each word in each situation.
// TODO: Can we get away without redefining the world?
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate speculate;
#[macro_use]
extern crate maplit;
#[macro_use(c)]
extern crate cute;
#[macro_use]
extern crate itertools;

// TODO: Can we get away without redefining the world?
pub mod bet;
pub mod die;
pub mod game;
pub mod hand;
pub mod player;
pub mod testing;
pub mod dict;
pub mod tile;

use crate::dict::ScrabbleDict;
use crate::bet::*;

use speculate::speculate;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;

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
    info!("Loading Scrabble dictionary...");
    let words = ScrabbleDict::words();
    info!("Generating all candidate strings...");
    words.iter().take(1).enumerate().map(|(i, w)| {
        info!("{}/{}: {}", i, &words.len(), w);
        all_sorted_substrings(&w)
    }).flatten().collect()
}

/// Creates a lookup table from word substrings
fn create_lookup(max_num_items: usize, num_trials: u32) -> HashMap<String, Vec<f64>> {
    let candidates = generate_sorted_candidates();
    info!("Computing for {} candidates", candidates.len());
    c! { s.clone() => probabilities(&s, max_num_items, num_trials), for s in generate_sorted_candidates() }
}

/// Computes the various probabilities of finding the given substring in each possible number of
/// items.
/// This returns a vec where index equates to the number of items we're searching in.
/// TODO: Do a separate MCMC to generate Palafico probabilities.
fn probabilities(s: &String, max_num_items: usize, num_trials: u32) -> Vec<f64> {
    info!("Computing for '{}'", s);
    (0..=max_num_items)
        .into_iter()
        .map(|n| monte_carlo(n as u32, &ScrabrudoBet::from_word(s).tiles, num_trials, false))
        .collect()
}

fn main() {
    pretty_env_logger::init();
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        info!("Please supply max_num_items and num_trials");
        return;
    }

    let max_num_items = args[1].parse::<usize>().unwrap();
    let num_trials = args[2].parse::<u32>().unwrap();

    let _ = create_lookup(max_num_items, num_trials);
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
                "aht".into(),
                "et".into(),
                "aet".into(),
                "aeht".into(),
                "e".into(),
                "ah".into(),
                "t".into(),
                "eh".into(),
                "ht".into(),
                "ae".into(),
                "at".into(),
                "aeh".into(),
                "h".into(),
                "eht".into(),
                "a".into()
            };
            let actual = all_sorted_substrings(&"hate".into());
            assert_eq!(expected, actual);
        }
    }
}
