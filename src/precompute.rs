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
extern crate clap;
extern crate rayon;
extern crate sstable;

// TODO: Can we get away without redefining the world?
pub mod bet;
pub mod dict;
pub mod die;
pub mod game;
pub mod hand;
pub mod player;
pub mod testing;
pub mod tile;

use crate::bet::*;
use crate::dict::*;

use clap::App;
use rayon::prelude::*;
use speculate::speculate;
use sstable::{Options, TableBuilder};
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::fs::OpenOptions;
use std::sync::Arc;
use std::sync::Mutex;

// TODO: I stole this code - find a library or something.
pub fn powerset<T: Clone>(slice: &[T]) -> Vec<Vec<T>> {
    let mut v: Vec<Vec<T>> = Vec::new();

    for mask in 0..(1 << slice.len()) {
        let mut ss: Vec<T> = vec![];
        let mut bitset = mask;
        while bitset > 0 {
            // isolate the rightmost bit to select one item
            let rightmost: u64 = bitset & !(bitset - 1);
            // turn the isolated bit into an array index
            let idx = rightmost.trailing_zeros();
            let item = (*slice.get(idx as usize).unwrap()).clone();
            ss.push(item);
            // zero the trailing bit
            bitset &= bitset - 1;
        }
        v.push(ss);
    }
    v
}

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
/// This is equivalent to the powerset of the characters of the word minus the empty word, sorted,
/// and filtered down to only those things that fit on the table.
fn all_sorted_substrings(word: &String, max_length: usize) -> HashSet<String> {
    let chars = &(word.chars().collect::<Vec<char>>())[..];
    powerset(chars)
        .par_iter()
        .map(|cs| cs.into_iter().collect::<String>())
        .filter(|w| w.len() > 0 && w.len() <= max_length)
        .map(|w| sort_word(&w))
        .collect()
}

/// Creates the lookup in a single iteration.
/// First we explode out via flat_map to all possible substrings, and then we map these to their
/// Monte Carlo probabilities.
fn create_lookup(
    lookup_path: &str,
    words: &HashSet<String>,
    max_num_items: usize,
    num_trials: u32,
) {
    // Expand out the dict to subwords.
    let word_counter = Arc::new(Mutex::new(0));
    let expanded_words = words
        .par_iter()
        .flat_map(|w| {
            *word_counter.lock().unwrap() += 1;
            info! {"{} / {} words expanded", word_counter.lock().unwrap(), words.len()};
            all_sorted_substrings(w, max_num_items)
        })
        .collect::<HashSet<String>>();
    info!("Created {} word expansions", expanded_words.len());

    // Compute all the probabilities and persist to disk.
    let prob_counter = Arc::new(Mutex::new(0));
    let mut probs = expanded_words
        .par_iter()
        .map(|s| {
            *prob_counter.lock().unwrap() += 1;
            info! {"{} / {} probs calculated", prob_counter.lock().unwrap(), expanded_words.len()};
            // Compute probs and encode
            let probs = bincode::serialize(&probabilities(&s, max_num_items, num_trials)).unwrap();
            (s, probs)
        })
        .collect::<Vec<(&String, Vec<u8>)>>();

    // Write the probs out to an SSTable.
    // First the keys need to be sorted.
    probs.sort_by(|a, b| a.0.cmp(&b.0));
    let lookup_file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(lookup_path)
        .unwrap();
    let mut builder = TableBuilder::new(Options::default(), lookup_file);
    for prob_row in probs {
        builder.add(prob_row.0.as_bytes(), &prob_row.1).unwrap();
    }
    builder.finish().unwrap();
}

/// Computes the various probabilities of finding the given substring in each possible number of
/// items.
/// This returns a vec where index equates to the number of items we're searching in.
/// TODO: Do a separate MCMC to generate Palafico probabilities.
fn probabilities(s: &String, max_num_items: usize, num_trials: u32) -> Vec<f64> {
    (0..=max_num_items)
        .into_iter()
        .map(|n| monte_carlo(n as u32, s, num_trials))
        .collect()
}

fn main() {
    pretty_env_logger::init();

    let matches = App::new("Scrabrudo Precomputation")
        .version("0.1")
        .about("Precomputes lookups for Scrabrudo")
        .author("Harry Askham")
        .args_from_usage(
            "-n, --num_tiles=[NUM_TILES] 'the max number of tiles to compute'
                        -t, --num_trials=[NUM_TRIALS] 'the number of trials to run'
                        -d, --dictionary_path=[DICTIONARY] 'the path to the .txt dict to use'
                        -l, --lookup_path=[LOOKUP] 'the path to the lookup DB to write'",
        )
        .get_matches();

    let mode = matches.value_of("mode").unwrap_or("scrabrudo");
    let num_players: usize = matches
        .value_of("num_players")
        .unwrap_or("2")
        .parse::<usize>()
        .unwrap();

    let dict_path = matches.value_of("dictionary_path").unwrap();
    dict::init_dict(dict_path);

    let num_tiles = matches
        .value_of("num_tiles")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let num_trials = matches
        .value_of("num_trials")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    let lookup_path = matches.value_of("lookup_path").unwrap();
    create_lookup(&lookup_path, &dict::dict(), num_tiles, num_trials);
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
            create_lookup("/tmp/lookup1.sstable", &hashset!{ "an".into() }, 5, 10000);
            dict::init_lookup("/tmp/lookup1.sstable");

            assert_eq!(3, dict::lookup_len());
            assert!(dict::lookup_has("a".into()));
            assert!(dict::lookup_has("n".into()));
            assert!(dict::lookup_has("an".into()));

            let probs = dict::lookup_probs("a".into()).unwrap();

            // We should always have for each amount of tiles, plus the zero-case.
            assert_eq!(6, probs.len());

            // Finding 'a' in 0 dice is always impossible.
            assert_eq!(0.0, probs[0]);

            // Always monotonically increasing as you add more dice
            info!("{:?}", probs);
            for i in 1..5 {
                assert!(probs[i] > probs[i - 1]);
            }
        }

        it "creates a larger lookup table" {
            create_lookup("/tmp/lookup2.sstable", &hashset!{ "bat".into(), "cat".into() }, 5, 10);
            dict::init_lookup("/tmp/lookup2.sstable");
            assert_eq!(11, dict::lookup_len());
        }
    }
}
