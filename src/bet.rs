/// Bet definitions and related logic.
use crate::dict;
use crate::dict::*;
use crate::die::*;
use crate::game::*;
use crate::hand::*;
use crate::player::*;
use crate::testing;
use crate::tile::*;

use probability::prelude::*;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;
use speculate::speculate;
use std::cmp::Ord;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::iter;

/// Trait implemented by any type of bet.
pub trait Bet: Ord + Clone + fmt::Display {
    type V: Holdable;

    /// Return all possible bets given the current game state.
    fn all(state: &GameState<Self>) -> Vec<Box<Self>>;

    /// Get all bets above this bet.
    fn all_above(&self, state: &GameState<Self>) -> Vec<Box<Self>> {
        // Generate all bets and filter down to only those which are greater than the one given.
        Self::all(state)
            .into_iter()
            .filter(|b| **b > *self)
            .collect::<Vec<Box<Self>>>()
    }

    /// Gets the smallest possible bet.
    fn smallest() -> Box<Self>;

    /// Pick the best bet from those available for a first go.
    /// TODO: Better than random choice from equally likely bets.
    fn best_first_bet(
        state: &GameState<Self>,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> Box<Self>;

    /// Whether or not this bet is correct given the items on the table.
    /// If 'exact' is true then it makes a Palafico evaluation.
    fn is_correct(&self, all_items: &Vec<Self::V>, exact: bool) -> bool;

    /// Get the probability of this bet being correct.
    fn prob(
        &self,
        state: &GameState<Self>,
        variant: ProbVariant,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> f64 {
        match variant {
            ProbVariant::Bet => self.bet_prob(state, player),
            ProbVariant::Perudo => self.perudo_prob(state, player),
            ProbVariant::Palafico => self.palafico_prob(state, player),
        }
    }

    /// Get the probability of the bet being correct.
    /// This is akin to the mass of this bet, plus all those with the same value and higher
    /// quantity.
    fn bet_prob(
        &self,
        state: &GameState<Self>,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> f64;

    /// Gets the probability that this bet is incorrect as far as the given player is concerned.
    /// This will always just be the negation of P(bet).
    /// Note that in some cases this is always going to win out - instead we need to take history
    /// into account.
    fn perudo_prob(
        &self,
        state: &GameState<Self>,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> f64 {
        1.0 - self.bet_prob(state, player)
    }

    /// Gets the probability that this bet is exactly correct as far as the given player is
    /// concerned.
    fn palafico_prob(
        &self,
        state: &GameState<Self>,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> f64;

    /// Gets all bets ordered by probability from the perspective of the given player.
    fn ordered_bets(
        state: &GameState<Self>,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> Vec<Box<Self>> {
        let mut bets = Self::all(state)
            .into_iter()
            // TODO: Remove awful hack to get around lack of Ord on f64 and therefore no sort().
            .map(|b| {
                (
                    (100000.0 * b.prob(state, ProbVariant::Bet, player.cloned())) as u64,
                    b,
                )
            })
            .collect::<Vec<(u64, Box<Self>)>>();
        bets.sort_by(|a, b| a.0.cmp(&b.0));
        bets.into_iter().map(|x| x.1).collect::<Vec<Box<Self>>>()
    }

    /// Return one of the highest probability bets from those given.
    fn best_bet_from(
        state: &GameState<Self>,
        player: Box<dyn Player<V = Self::V, B = Self>>,
        bets: Vec<Box<Self>>,
    ) -> Box<Self> {
        let max_prob = bets[bets.len() - 1].prob(state, ProbVariant::Bet, player.cloned());
        let best_bets = bets
            .into_iter()
            .filter(|b| b.prob(state, ProbVariant::Bet, player.cloned()) == max_prob)
            .collect::<Vec<Box<Self>>>();
        let mut rng = thread_rng();
        best_bets.choose(&mut rng).unwrap().clone()
    }
}

/// The different types of Bet one can make in Perudo.
/// Used to modulate how we perform probability calculations.
pub enum ProbVariant {
    Bet,
    Perudo,
    Palafico,
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct PerudoBet {
    pub value: Die,
    pub quantity: usize,
}

impl Bet for PerudoBet {
    type V = Die;

    fn all(state: &GameState<Self>) -> Vec<Box<Self>> {
        iproduct!(Die::all().into_iter(), 1..=state.total_num_items)
            .map(|(value, quantity)| {
                Box::new(PerudoBet {
                    value: value,
                    quantity: quantity,
                })
            })
            .collect::<Vec<Box<PerudoBet>>>()
    }

    fn smallest() -> Box<Self> {
        Box::new(Self {
            quantity: 0,
            value: Die::Two,
        })
    }

    /// TODO: Too much cloning here.
    fn best_first_bet(
        state: &GameState<Self>,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> Box<Self> {
        let bets = Self::ordered_bets(state, player.cloned())
            .into_iter()
            .filter(|b| b.value != Die::One)
            .collect::<Vec<Box<Self>>>();
        Self::best_bet_from(state, player, bets)
    }

    fn is_correct(&self, all_items: &Vec<Self::V>, exact: bool) -> bool {
        unimplemented!("This is currently handed in game.rs");
    }

    fn palafico_prob(
        &self,
        state: &GameState<Self>,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> f64 {
        let guaranteed_quantity = player.num_logical_items(self.value.clone());
        if guaranteed_quantity > self.quantity {
            return 0.0;
        }

        let trial_p: f64 = if self.value == Die::One {
            1.0 / 6.0
        } else {
            1.0 / 3.0
        };
        let num_other_dice = state.total_num_items - player.num_items();
        // This is a single Binomial trial - what's the probability of finding the rest of the dice
        // in the remaining dice.
        // TODO: &This occasionally crashes in the mass() func, possibly due to overflow.
        Binomial::new(num_other_dice, trial_p).mass(self.quantity - guaranteed_quantity)
    }

    fn bet_prob(
        &self,
        state: &GameState<Self>,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> f64 {
        // If we have the bet in-hand, then we're good; otherwise we only have to look for the diff
        // in the other probabilities.
        let guaranteed_quantity = player.num_logical_items(self.value.clone());
        if self.quantity <= guaranteed_quantity {
            return 1.0;
        }

        // TODO: Reframe the below as 1 minus the CDF of up to the bet.
        // Since we say the bet is correct if there are really n or higher.
        // We want 1 minus the probability there are less than n.
        // So that's 1 - cdf(n - 1)
        let trial_p: f64 = if self.value == Die::One {
            1.0 / 6.0
        } else {
            1.0 / 3.0
        };
        let num_other_dice = state.total_num_items - player.num_items();
        ((self.quantity - guaranteed_quantity)..=num_other_dice)
            .map(|q| Binomial::new(num_other_dice, trial_p).mass(q))
            .sum::<f64>()
    }
}

impl fmt::Display for PerudoBet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {:?}s", self.quantity, self.value)
    }
}

impl Ord for PerudoBet {
    fn cmp(&self, other: &PerudoBet) -> Ordering {
        if self.value == Die::One && other.value == Die::One {
            // If both are ace, then just compare the values.
            self.quantity.cmp(&other.quantity)
        } else if self.value == Die::One {
            // If this is ace, compare its double.
            // We don't +1 here as we want 1x1 to be less than 3x2, not equal.
            // We also do not define equality here in order to enforce unidirectionality of
            // ace-lifting.
            if self.quantity * 2 >= other.quantity {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        } else if other.value == Die::One {
            if other.quantity * 2 >= self.quantity {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        } else if (self.value == other.value && self.quantity > other.quantity)
            || (self.value > other.value && self.quantity >= other.quantity)
        {
            // If we've increased the die quantity only then the bet is larger.
            Ordering::Greater
        } else if self.quantity > other.quantity {
            // It is sufficient to increase the quanntity - we do not then care about the value,
            // you can bid anything.
            Ordering::Greater
        } else if self.value == other.value && self.quantity == other.quantity {
            Ordering::Equal
        } else {
            Ordering::Less
        }
    }
}

impl PartialOrd for PerudoBet {
    fn partial_cmp(&self, other: &PerudoBet) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A single bet consisting of Scrabble tiles.
#[derive(Debug, Clone, Hash)]
pub struct ScrabrudoBet {
    /// The list of tiles that make up the proposed word.
    pub tiles: Vec<Tile>,
}

impl Bet for ScrabrudoBet {
    type V = Tile;

    fn all(state: &GameState<Self>) -> Vec<Box<Self>> {
        dict::words_with_max_length(state.total_num_items)
            .into_iter()
            .map(|w| Box::new(Self::from_word(&w)))
            .collect()
    }

    fn smallest() -> Box<Self> {
        Box::new(Self { tiles: vec![] })
    }

    fn best_first_bet(
        state: &GameState<Self>,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> Box<Self> {
        // TODO: If we make a distinction for the first bet here then we should incorporate it
        // here.
        let bets = Self::ordered_bets(state, player.cloned());
        Self::best_bet_from(state, player, bets)
    }

    fn is_correct(&self, all_items: &Vec<Self::V>, exact: bool) -> bool {
        // We cannot check against the fucking dictionary here!
        // If we do we omit all the substrinngs that aren't in the dict.
        // This simply says: can you make this bet with the tiles.

        // We need to extract the blanks here and kind of "cout them down" as we find the bet is
        // missing letters. If we run out of blanks, we lose.
        let tile_counts = count_map(&self.tiles);
        let all_tile_counts = count_map(&all_items);
        let num_blanks = all_items.iter().filter(|t| *t == &Tile::Blank).count();

        if exact {
            // Palafico pathway
            let mut num_chars_missing = 0;
            let mut any_over = false;
            for (tile, count) in &tile_counts {
                let actual_count = match all_tile_counts.get(tile) {
                    Some(c) => *c,
                    None => 0,
                };
                if actual_count > *count {
                    any_over = true;
                    break;
                } else if actual_count < *count {
                    num_chars_missing += *count - actual_count;
                }
            }

            !any_over && (num_chars_missing <= num_blanks)
        } else {
            // Perudo pathway
            let mut num_chars_missing = 0;
            for (tile, count) in &tile_counts {
                let actual_count = match all_tile_counts.get(tile) {
                    Some(c) => *c,
                    None => 0,
                };
                if actual_count < *count {
                    num_chars_missing += *count - actual_count;
                }
            }
            num_chars_missing <= num_blanks
        }
    }

    fn bet_prob(
        &self,
        state: &GameState<Self>,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> f64 {
        // Rough algorithm for calculating probability of bet correctness:
        // for e.g. target = [A, T, T, A, C, K], n = 20, hand = [X, X, A, K]
        // Take the difference of the target and the hand. This leaves the letters we seek from the
        // wider tile pool.
        //
        // [A T T A C K] - [X X A K] = [T T A C]
        //
        // This is no longer "3" or "NOT 3" as in Perudo - this is a multinomial probability.
        // e.g. We need at least 2xT, 1xA, 1xC from the same pool of dice.
        // We can therefore enumerate all winning cases as the set of tuple counts:
        // SUM { P(C=c A=a T=t; n=16, p=[1/26..]) | c>=1 a>=1 t>=2, c+a+t<=16 }
        //
        // However this hugely explodes the problem as we also need to account for all numbers of
        // letters other than C A and T. Doing this triple generation naively will result in huge
        // numbers of candidates.
        //
        // A huge precomputed table from e.g. [w = ATTACK,ATTAC,ATTAK... -> P(w)] is plausible if
        // the computation takes very long, but not ideal.
        //
        // Generating combos is also quite cumbersome.
        //
        // We could also use Monte Carlo simulation here, as follows:
        // - Draw 16 tiles
        // - Are all tiles there?
        // - Count yes's, no's, repeat many times, divide
        //
        // However, doing Monte Carlo for every possible word in the list will take forever.
        // Could look at Monte Carlo precomputation...

        // First get the set of tiles we need to find.
        let mut tiles_to_find = self.tiles.clone();
        for tile in player.items() {
            match tiles_to_find.binary_search(tile) {
                Ok(i) => {
                    tiles_to_find.remove(i);
                }
                Err(_) => (),
            };
        }

        // Get the number of tiles we have to search in.
        let num_tiles = state.total_num_items - player.num_items();

        // TODO: "Believe" a certain number of tiles here, potentially only from the bet of the
        // next player to play.
        // TODO: Plug in a strategy here - right now we believe everything the last person said.
        let belief_tiles = match state.history.last() {
            None => vec![],
            Some(historical_bet) => historical_bet.bet.tiles.clone()
        };

        // Remove all the belief tiles from that which we have to find.
        for tile in belief_tiles {
            match tiles_to_find.binary_search(&tile) {
                Ok(i) => {
                    tiles_to_find.remove(i);
                }
                Err(_) => (),
            };
        }

        // If we have all the tiles, it's a guaranteed hit.
        if tiles_to_find.is_empty() {
            return 1.0;
        }


        // Sort the tiles to find and turn into a word to match the lookup.
        tiles_to_find.sort_by(|a, b| a.char().cmp(&b.char()));
        let substring = tiles_to_find
            .into_iter()
            .map(|t| t.char())
            .collect::<String>();
        if !LOOKUP.lock().unwrap().contains_key(&substring) {
            0.0 // If we somehow didn't compute this length yet then 0.0
                // We can prob remove the above
        } else {
            LOOKUP.lock().unwrap().get(&substring).unwrap()[num_tiles]
        }
    }

    fn palafico_prob(
        &self,
        _state: &GameState<Self>,
        _player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> f64 {
        // TODO: Not quite ready for full Palafico yet, we have the exact-find prob but we also
        // need the logic to make sure our own hand doesn't spill over.
        0.0
    }
}

/// Gets a map of tiles to their counts.
pub fn count_map(tiles: &Vec<Tile>) -> HashMap<&Tile, usize> {
    let mut count_map = HashMap::new();
    for tile in tiles {
        let count = count_map.entry(tile).or_insert(0 as usize);
        *count += 1;
    }
    count_map
}

/// Runs MC simulation to get rough probability of success.
/// TODO: Move to a monte_carlo module.
pub fn monte_carlo(n: u32, word: &String, num_trials: u32) -> f64 {
    if n == 0 {
        // Cannot find a word in no tiles.
        return 0.0;
    }

    let bet = ScrabrudoBet::from_word(word);

    let mut success = 0;
    for i in 0..num_trials {
        let all_tiles = Hand::<Tile>::new(n).items;
        if bet.is_correct(&all_tiles, false) {
            success += 1;
        }
    }

    success as f64 / num_trials as f64
}

impl ScrabrudoBet {
    pub fn from_word(word: &String) -> Self {
        let tiles = word
            .chars()
            .map(|c| Tile::from_char(c))
            .collect::<Vec<Tile>>();
        Self { tiles }
    }

    pub fn as_word(&self) -> String {
        self.tiles.iter().map(|t| t.char()).collect()
    }

    pub fn score(&self) -> u32 {
        self.tiles.iter().map(|t| t.score()).sum()
    }
}

impl fmt::Display for ScrabrudoBet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "'{}'", self.as_word())
    }
}

impl Ord for ScrabrudoBet {
    fn cmp(&self, other: &ScrabrudoBet) -> Ordering {
        if self.tiles.len() == other.tiles.len() {
            // If the same length, lexicographic ordering wins.
            self.as_word().cmp(&other.as_word())
        } else {
            // Otherwise the longer word wins.
            self.tiles.len().cmp(&other.tiles.len())
        }
    }
}

impl PartialOrd for ScrabrudoBet {
    fn partial_cmp(&self, other: &ScrabrudoBet) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ScrabrudoBet {
    // Define equality by containing equal tiles.
    fn eq(&self, other: &ScrabrudoBet) -> bool {
        let mut self_tiles = self.tiles.clone();
        let mut other_tiles = other.tiles.clone();
        self_tiles.sort_by(|a, b| a.char().cmp(&b.char()));
        other_tiles.sort_by(|a, b| a.char().cmp(&b.char()));
        self_tiles == other_tiles
    }
}

impl Eq for ScrabrudoBet {}

speculate! {
    before {
        testing::set_up();
    }

    describe "scrabrudo bets" {
        it "converts bet to word and back" {
            let bet = ScrabrudoBet{
                tiles: vec![Tile::C, Tile::A, Tile::T],
            };
            assert_eq!("cat", bet.as_word());
            assert_eq!(ScrabrudoBet::from_word(&"cat".into()), bet);
        }

        it "can load all bets for a certain number of tiles" {
            let bets = ScrabrudoBet::all(&GameState::<ScrabrudoBet>{
                total_num_items: 4,
                num_items_per_player: vec![4],
                history: vec![],
            });
            assert_eq!(4971, bets.len());
            for bet in bets {
                assert!(bet.tiles.len() <= 4);
            }
        }

        /* Disabled due to slow execution.
        it "can load all bets for a large number of tiles" {
            let bets = ScrabrudoBet::all(&GameState<Self>{
                total_num_items: 30,
                num_items_per_player: vec![30]
            });
            assert_eq!(172820, bets.len());
            for bet in bets {
                assert!(bet.tiles.len() <= 30);
            }
        }
        */

        it "orders bets correctly" {
            // These happen to be correct whether score or length ordered.
            let bets = vec![
                ScrabrudoBet::from_word(&"a".into()),
                ScrabrudoBet::from_word(&"at".into()),
                ScrabrudoBet::from_word(&"cat".into()),
                ScrabrudoBet::from_word(&"chat".into()),
                ScrabrudoBet::from_word(&"zhat".into()),
                ScrabrudoBet::from_word(&"chart".into()),
                ScrabrudoBet::from_word(&"chariot".into()),
                ScrabrudoBet::from_word(&"chariots".into()),
            ];

            for i in 0..(bets.len() - 1) {
                assert_eq!(bets[i], bets[i].clone());
                assert!(bets[i] < bets[i + 1]);
                assert!(bets[i + 1] > bets[i]);
            }
        }

        it "treats anagram bets equally" {
            assert_eq!(ScrabrudoBet::from_word(&"cat".into()), ScrabrudoBet::from_word(&"act".into()));
            assert_eq!(ScrabrudoBet::from_word(&"desserts".into()), ScrabrudoBet::from_word(&"stressed".into()));
            assert_ne!(ScrabrudoBet::from_word(&"dessert".into()), ScrabrudoBet::from_word(&"stressed".into()));
        }

        it "checks bet correctness" {
            assert!(ScrabrudoBet::from_word(&"cat".into()).is_correct(&vec![Tile::C, Tile::A, Tile::T], false));
            assert!(ScrabrudoBet::from_word(&"cat".into()).is_correct(&vec![Tile::C, Tile::A, Tile::Blank], false));
            assert!(!ScrabrudoBet::from_word(&"cat".into()).is_correct(&vec![Tile::C, Tile::Blank], false));
            assert!(ScrabrudoBet::from_word(&"cat".into()).is_correct(&vec![Tile::C, Tile::A, Tile::T, Tile::H], false));
            assert!(ScrabrudoBet::from_word(&"chat".into()).is_correct(&vec![Tile::Blank, Tile::A, Tile::T, Tile::H], false));
        }

        it "checks exact bet correctness" {
            // TODO: implement
        }
    }

    describe "perudo bets" {
        fn bet(v: Die, q: usize) -> Box<PerudoBet> {
            Box::new(PerudoBet {
                value: v,
                quantity: q,
            })
        }

        it "orders bets correctly" {
            let bet_1 = bet(Die::Two, 1);
            let bet_2 = bet(Die::Two, 2);
            let bet_3 = bet(Die::Two, 3);
            let bet_4 = bet(Die::Three, 3);
            let bet_5 = bet(Die::Three, 4);
            let bet_6 = bet(Die::Two, 5);
            let bet_7 = bet(Die::Two, 6);
            let bet_8 = bet(Die::Three, 8);
            let bet_9 = bet(Die::Six, 10);

            assert_eq!(bet_1, bet_1.clone());

            assert!(bet_1 < bet_2);
            assert!(bet_2 < bet_3);
            assert!(bet_3 < bet_4);
            assert!(bet_4 < bet_5);
            assert!(bet_5 < bet_6);
            assert!(bet_6 < bet_7);
            assert!(bet_7 < bet_8);
            assert!(bet_8 < bet_9);

            assert!(bet_2 > bet_1);
            assert!(bet_3 > bet_2);
            assert!(bet_4 > bet_3);
            assert!(bet_5 > bet_4);
            assert!(bet_6 > bet_5);
            assert!(bet_7 > bet_6);
            assert!(bet_8 > bet_7);
            assert!(bet_9 > bet_8);
        }

        it "orders ace bets correctly" {
            let bet_1 = bet(Die::Two, 1);
            let bet_2 = bet(Die::One, 1);
            let bet_3 = bet(Die::Two, 3);
            let bet_4 = bet(Die::Two, 4);
            let bet_5 = bet(Die::One, 2);
            let bet_6 = bet(Die::One, 3);
            let bet_7 = bet(Die::Five, 7);
            let bet_8 = bet(Die::One, 4);
            let bet_9 = bet(Die::Six, 9);

            assert!(bet_1 < bet_2);
            assert!(bet_2 < bet_3);
            assert!(bet_3 < bet_4);
            assert!(bet_4 < bet_5);
            assert!(bet_5 < bet_6);
            assert!(bet_6 < bet_7);
            assert!(bet_7 < bet_8);
            assert!(bet_8 < bet_9);

            assert!(bet_2 > bet_1);
            assert!(bet_3 > bet_2);
            assert!(bet_4 > bet_3);
            assert!(bet_5 > bet_4);
            assert!(bet_6 > bet_5);
            assert!(bet_7 > bet_6);
            assert!(bet_8 > bet_7);
            assert!(bet_9 > bet_8);
        }

        it "generates all above" {
            let original = PerudoBet {
                value: Die::Two,
                quantity: 1,
            };
            assert_eq!(
                vec![
                    bet(Die::One, 1),
                    bet(Die::One, 2),
                    bet(Die::Two, 2),
                    bet(Die::Three, 1),
                    bet(Die::Three, 2),
                    bet(Die::Four, 1),
                    bet(Die::Four, 2),
                    bet(Die::Five, 1),
                    bet(Die::Five, 2),
                    bet(Die::Six, 1),
                    bet(Die::Six, 2),
                ],
                original.all_above(&GameState::<PerudoBet>{
                    total_num_items: 2,
                    num_items_per_player: vec![1, 1],
                    history: vec![],
                }));
        }

        fn approx(x: f64, y: f64) {
            if (x - y).abs() > 0.001 {
                panic!("{} != {}", x, y);
            }
        }

        it "computes probability for bets" {
            // Create a player with a few of each.
            let player = Box::new(PerudoPlayer {
                id: 0,
                human: false,
                hand: Hand::<Die> {
                    items: vec![
                        Die::One ,
                        Die::Two ,
                        Die::Three ,
                        Die::Four ,
                        Die::Five
                    ],
                },
            });

            let state = &GameState::<PerudoBet>{
                total_num_items: 6,
                num_items_per_player: vec![5, 1],
                history: vec![],
            };

            // Bets on Ones, given one in the hand.
            approx(1.0, bet(Die::One, 0).prob(state, ProbVariant::Bet, player.cloned()));
            approx(1.0, bet(Die::One, 1).prob(state, ProbVariant::Bet, player.cloned()));
            approx(1.0 / 6.0, bet(Die::One, 2).prob(state, ProbVariant::Bet, player.cloned()));

            // We have two 2s in the hand already.
            approx(1.0, bet(Die::Two, 0).prob(state, ProbVariant::Bet, player.cloned()));
            approx(1.0, bet(Die::Two, 1).prob(state, ProbVariant::Bet, player.cloned()));
            approx(1.0, bet(Die::Two, 2).prob(state, ProbVariant::Bet, player.cloned()));
            approx(1.0 / 3.0, bet(Die::Two, 3).prob(state, ProbVariant::Bet, player.cloned()));

            // TODO: More tests for the prob-calcs.
        }
    }

    describe "monte carlo" {
        it "approximates the chance of a bet" {
            let p = monte_carlo(20, &"cat".into(), 10000);

            // We should definitely find it a bunch of times in 20 die.
            assert!(p > 0.0);
        }
    }
}
