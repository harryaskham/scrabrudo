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
extern crate bincode;
#[macro_use]
extern crate lazy_static;

// TODO: Can we get away without redefining the world?
pub mod bet;
pub mod die;
pub mod game;
pub mod hand;
pub mod player;
pub mod testing;
pub mod dict;
pub mod tile;

use crate::dict::*;
use crate::bet::*;

use speculate::speculate;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs::File;

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
///
/// This is equivalent to the powerset of the characters of the word minus the empty word, sorted.
///
/// TODO: This is excruciatingly slow for longer words
fn all_sorted_substrings(word: &String, max_length: usize) -> HashSet<String> {
    if word.len() == 1 {
        return hashset! { sort_word(word) };
    }

    let mut substrings = hashset! { sort_word(word) };

    for i in 0..word.len() {
        let mut word_without = word.clone();
        word_without.remove(i);
        let substrings_without = all_sorted_substrings(&word_without, max_length);
        substrings.extend(substrings_without);
    }

    substrings.into_iter().filter(|s| s.len() <= max_length).collect()
}

/// Generates all possible valid candidate strings.
/// This is all words plus all non-contiguous substrings of those words.
fn generate_sorted_candidates(words: &HashSet<String>, max_length: usize) -> HashSet<String> {
    // TODO: Support words of more than 5 in length.
    let words: HashSet<String> = words.clone().into_iter().filter(|w| w.len() <= 5).collect();

    info!("Generating all candidate strings...");
    words.iter().enumerate().map(|(i, w)| {
        if i % 100 == 0 { info!("{} / {}: {}", i, &words.len(), w); }
        all_sorted_substrings(&w, max_length)
    }).flatten().collect()
}

/// Creates a lookup table from word substrings
fn create_lookup(words: &HashSet<String>, max_num_items: usize, num_trials: u32) -> HashMap<String, Vec<f64>> {
    let candidates = generate_sorted_candidates(words, max_num_items);
    info!("Computing for {} candidates", candidates.len());
    c! { 
        s.clone() => {
            if i % 100 == 0 { info!("{} / {}: {}", i, &candidates.len(), s); }
            probabilities(&s, max_num_items, num_trials)
        },
        for (i, s) in candidates.iter().enumerate()
    }
}

/// Computes the various probabilities of finding the given substring in each possible number of
/// items.
/// This returns a vec where index equates to the number of items we're searching in.
/// TODO: Do a separate MCMC to generate Palafico probabilities.
fn probabilities(s: &String, max_num_items: usize, num_trials: u32) -> Vec<f64> {
    (0..=max_num_items)
        .into_iter()
        .map(|n| monte_carlo(n as u32, &ScrabrudoBet::from_word(s).tiles, num_trials, false))
        .collect()
}

/// Save the lookup to disk.
fn persist_lookup(lookup: &HashMap<String, Vec<f64>>, path: &str) {
    info!("Saving the lookup table as {}...", path);

    let mut file = File::create(path).unwrap();
    bincode::serialize_into(&mut file, lookup).unwrap();

    info!("Saved the lookup table as {}", path);
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

    let lookup = create_lookup(&SCRABBLE_DICT.words, max_num_items, num_trials);
    persist_lookup(&lookup, "data/lookup");
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
            let actual = all_sorted_substrings(&"hate".into(), 4);
            assert_eq!(expected, actual);
        }

        it "enforces a max length" {
            let expected = hashset! {
                "et".into(),
                "e".into(),
                "ah".into(),
                "t".into(),
                "eh".into(),
                "ht".into(),
                "ae".into(),
                "at".into(),
                "h".into(),
                "a".into()
            };
            let actual = all_sorted_substrings(&"hate".into(), 2);
            assert_eq!(expected, actual);
        }
    }

    describe "lookup generation" {
        it "creates a small lookup table" {
            let lookup = create_lookup(&hashset!{ "a".into() }, 5, 10000);
            assert_eq!(1, lookup.len());
            assert!(lookup.contains_key("a".into()));

            let probs = lookup.get("a".into()).unwrap();

            // We should always have for each amount of tiles, plus the zero-case.
            assert_eq!(6, probs.len());

            // Finding 'a' in 0 dice is always impossible.
            assert_eq!(0.0, probs[0]);

            // Always monotonically increasing as you add more dice
            for i in 1..5 {
                assert!(probs[i] > probs[i - 1]);
            }
        }

        it "creates a larger lookup table" {
            let lookup = create_lookup(&hashset!{ "bat".into(), "cat".into() }, 5, 10);
            let actual_keys = lookup.keys().map(|k| k.clone()).collect::<HashSet<String>>();
            let expected_keys: HashSet<String> = hashset! {
                "abt".into(),
                "act".into(),
                "ab".into(),
                "ac".into(),
                "at".into(),
                "bt".into(),
                "ct".into(),
                "a".into(),
                "b".into(),
                "c".into(),
                "t".into(),
            };
            assert_eq!(expected_keys, actual_keys);
        }
    }
}
