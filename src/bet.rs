use crate::die::*;
/// Bet definitions and related logic.
use crate::game::*;
use crate::hand::*;
use crate::player::*;
use crate::testing;
use crate::dict::*;
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
    fn all(state: &GameState) -> Vec<Box<Self>>;

    /// Get all bets above this bet.
    fn all_above(&self, state: &GameState) -> Vec<Box<Self>> {
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
        state: &GameState,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> Box<Self>;

    /// Get the probability of this bet being correct.
    fn prob(
        &self,
        state: &GameState,
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
    fn bet_prob(&self, state: &GameState, player: Box<dyn Player<V = Self::V, B = Self>>) -> f64;

    /// Gets the probability that this bet is incorrect as far as the given player is concerned.
    /// This will always just be the negation of P(bet).
    fn perudo_prob(
        &self,
        state: &GameState,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> f64 {
        1.0 - self.bet_prob(state, player)
    }

    /// Gets the probability that this bet is exactly correct as far as the given player is
    /// concerned.
    fn palafico_prob(
        &self,
        state: &GameState,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> f64;

    /// Gets all bets ordered by probability from the perspective of the given player.
    fn ordered_bets(
        state: &GameState,
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
        state: &GameState,
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

    fn all(state: &GameState) -> Vec<Box<Self>> {
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
        state: &GameState,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> Box<Self> {
        let bets = Self::ordered_bets(state, player.cloned())
            .into_iter()
            .filter(|b| b.value != Die::One)
            .collect::<Vec<Box<Self>>>();
        Self::best_bet_from(state, player, bets)
    }

    fn palafico_prob(
        &self,
        state: &GameState,
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

    fn bet_prob(&self, state: &GameState, player: Box<dyn Player<V = Self::V, B = Self>>) -> f64 {
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

    fn all(state: &GameState) -> Vec<Box<Self>> {
        ScrabbleDict::words_with_max_length(state.total_num_items)
            .into_iter()
            .map(|w| Box::new(Self::from_word(w)))
            .collect()
    }

    fn smallest() -> Box<Self> {
        Box::new(Self { tiles: vec![] })
    }

    fn best_first_bet(
        state: &GameState,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> Box<Self> {
        // TODO: If we make a distinction for the first bet here then we should incorporate it
        // here.
        let bets = Self::ordered_bets(state, player.cloned());
        Self::best_bet_from(state, player, bets)
    }

    fn bet_prob(&self, state: &GameState, player: Box<dyn Player<V = Self::V, B = Self>>) -> f64 {
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

        // If we have all the tiles, it's a guaranteed hit.
        if tiles_to_find.is_empty() {
            return 1.0;
        }

        /* DISABLING THIS as it's destined to fail without an efficient Multinomial CDF computer.
        // Create a map from tile to count.
        let mut counts_to_find = count_map(&tiles_to_find);

        // We need to search for these tiles in the total unseen remaining tiles.
        let num_tiles = state.total_num_items - player.num_items();

        // Define the per-class probability for each tile.
        // The class probabilities are all equal here.
        // TODO: If we introduce unequal letter probabilities then this needs updating too.
        let p = iter::repeat(1.0 / 26.0).take(26);

        // We now generate the acceptable lower per-class counts for each tile.
        // TODO: Probably there will always be more unsuccessful counts, invert?
        let lower_bounds = (0..26)
            .into_iter()
            .map(|i| counts_to_find.get(&Tile::from_usize(i as usize)).unwrap())
            .collect::<Vec<&usize>>();

        // Get all possible combinations of counts.
        // e.g. All 26-tuples that sum to the remaining
        let combos = get_combos(26, num_tiles);

        // Now remove any violating combos.
        */

        let num_tiles = state.total_num_items - player.num_items();
        monte_carlo(num_tiles as u32, &tiles_to_find, 10, false)
    }

    fn palafico_prob(
        &self,
        state: &GameState,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> f64 {
        // TODO: This will stop the computer from ever considering Palafico but we should revisit
        // when decided upon a meaning for the rule.
        // If we decide it's no-duplicates-allowed then we have
        //
        // P(C=1 A=1 T=2 | n=16, p=[1/26..])
        //
        // But this still maps on to a large number of probabilities to compute given the other
        // possible values this can take.

        /* NOTE this also still doesn't work - we need to check in our own hand to see if we have
         * breaking duplicates.
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

        // If we have all the tiles, it's a guaranteed hit.
        if tiles_to_find.is_empty() {
            return 1.0;
        }

        let num_tiles = state.total_num_items - player.num_items();
        monte_carlo(num_tiles as u32, &tiles_to_find, 10, true)
        */
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

/// Runs a crappy MC simulation to get rough probability of success.
/// TODO: If keeping this approach, can multi-thread it easily.
fn monte_carlo(n: u32, tiles: &Vec<Tile>, num_trials: u32, exact: bool) -> f64 {
    let mut success = 0;
    let mut failure = 0;
    let tile_counts = count_map(tiles);
    for _ in 0..num_trials {
        let all_tiles = Hand::<Tile>::new(n).items;
        let all_tile_counts = count_map(&all_tiles);
        let mut okay = true;
        for (tile, count) in &tile_counts {
            let actual_count = match all_tile_counts.get(tile) {
                Some(c) => *c,
                None => 0,
            };
            if exact {
                if actual_count != *count {
                    okay = false;
                    break;
                }
            } else {
                if actual_count < *count {
                    okay = false;
                    break;
                }
            }
        }
        if okay {
            success += 1;
        } else {
            failure += 1
        }
    }
    success as f64 / num_trials as f64
}

/// Generates all combinations of values of length n that sum to sum.
fn get_combos(n: usize, sum: usize) -> Vec<Vec<usize>> {
    if n == 1 {
        return vec![vec![sum]];
    }

    // Recursively generate these tree-like from the current count.
    let mut all_combos = Vec::new();
    for i in 0..=sum {
        let combos = get_combos(n - 1, sum - i);
        for mut combo in combos {
            combo.push(i);
            all_combos.push(combo);
        }
    }

    all_combos
}

impl ScrabrudoBet {
    pub fn from_word(word: String) -> Self {
        let tiles = word
            .chars()
            .map(|c| Tile::from_char(c))
            .collect::<Vec<Tile>>();
        Self { tiles }
    }

    pub fn as_word(&self) -> String {
        self.tiles.iter().map(|t| t.char()).collect()
    }
}

impl fmt::Display for ScrabrudoBet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "'{}'", self.as_word())
    }
}

impl Ord for ScrabrudoBet {
    fn cmp(&self, other: &ScrabrudoBet) -> Ordering {
        // TODO: This implements simple length-ordering - a raise must be longer, in other words.
        // Experiment with other scoring systems.
        self.tiles.len().cmp(&other.tiles.len())
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
            assert_eq!(ScrabrudoBet::from_word("cat".into()), bet);
        }

        it "can load all bets for a certain number of tiles" {
            let bets = ScrabrudoBet::all(&GameState{
                total_num_items: 4,
                num_items_per_player: vec![4]
            });
            assert_eq!(4971, bets.len());
            for bet in bets {
                assert!(bet.tiles.len() <= 4);
            }
        }

        /* Disabled due to slow execution.
        it "can load all bets for a large number of tiles" {
            let bets = ScrabrudoBet::all(&GameState{
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
            let bets = vec![
                ScrabrudoBet::from_word("a".into()),
                ScrabrudoBet::from_word("at".into()),
                ScrabrudoBet::from_word("cat".into()),
                ScrabrudoBet::from_word("chat".into()),
                ScrabrudoBet::from_word("chart".into()),
                ScrabrudoBet::from_word("chariot".into()),
                ScrabrudoBet::from_word("chariots".into()),
            ];

            for i in 0..(bets.len() - 1) {
                assert_eq!(bets[i], bets[i].clone());
                assert!(bets[i] < bets[i + 1]);
                assert!(bets[i + 1] > bets[i]);
            }
        }

        it "generates all above" {
            let original = ScrabrudoBet::from_word("cat".into());
            let bets = original.all_above(&GameState{
                total_num_items: 4,
                num_items_per_player: vec![2, 2],
            });
            // This should give us all 4-letter words for now.
            assert_eq!(3903, bets.len());
            for bet in bets {
                assert_eq!(4, bet.tiles.len());
            }
        }

        it "treats anagram bets equally" {
            assert_eq!(ScrabrudoBet::from_word("cat".into()), ScrabrudoBet::from_word("act".into()));
            assert_eq!(ScrabrudoBet::from_word("desserts".into()), ScrabrudoBet::from_word("stressed".into()));
            assert_ne!(ScrabrudoBet::from_word("dessert".into()), ScrabrudoBet::from_word("stressed".into()));
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
                original.all_above(&GameState{
                    total_num_items: 2,
                    num_items_per_player: vec![1, 1],
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

            let state = &GameState{
                total_num_items: 6,
                num_items_per_player: vec![5, 1],
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

    describe "combo generation" {
        it "generates many possible combos" {
            let combos = get_combos(3, 2);
            assert_eq!(6, combos.len());
        }

        /* Disabled due to long runtime :( remove when a better solution is found.
        it "generates all the combos for a plausible early-game word setup" {
            let combos = get_combos(26, 20);
            assert_eq!(0, combos.len());
        }
        */
    }

    describe "monte carlo" {
        it "approximates the chance of a bet" {
            let p = monte_carlo(20, &vec![Tile::C, Tile::A, Tile::T], 10000, false);
            assert!(p > 0.10);
            assert!(p < 0.30);
        }
    }
}
