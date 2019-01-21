use crate::die::*;
/// Bet definitions and related logic.
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
use std::collections::HashSet;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Result};

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
    fn perudo_prob(&self, state: &GameState, player: Box<dyn Player<V = Self::V, B = Self>>)
        -> f64;

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

    fn smallest() -> Box<Self> {
        Box::new(Self {
            quantity: 0,
            value: Die::Two,
        })
    }

    /// TODO: Better than random choice from equally likely bets.
    /// TODO: Too much cloning here.
    fn best_first_bet(
        state: &GameState,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> Box<Self> {
        let bets = Self::first_bets(state, player.cloned());
        let max_prob = bets[bets.len() - 1].prob(state, ProbVariant::Bet, player.cloned());
        let best_bets = bets
            .into_iter()
            .filter(|b| b.prob(state, ProbVariant::Bet, player.cloned()) == max_prob)
            .collect::<Vec<Box<Self>>>();
        let mut rng = thread_rng();
        best_bets.choose(&mut rng).unwrap().clone()
    }

    fn perudo_prob(
        &self,
        state: &GameState,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> f64 {
        1.0 - self.bet_prob(state, player)
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

impl PerudoBet {
    /// Get the allowed first bets - everything but ones.
    /// Bets are ordered by their probability of occuring.
    fn first_bets(state: &GameState, player: Box<dyn Player<V = Die, B = Self>>) -> Vec<Box<Self>> {
        Self::ordered_bets(state, player)
            .into_iter()
            .filter(|b| b.value != Die::One)
            .collect::<Vec<Box<Self>>>()
    }

    /// All the valid bets without aces, for first-turn purposes.
    pub fn all_without_ones(state: &GameState) -> Vec<Box<Self>> {
        PerudoBet::all(state)
            .into_iter()
            .filter(|b| b.value != Die::One)
            .collect::<Vec<Box<PerudoBet>>>()
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
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct ScrabrudoBet {
    /// The list of tiles that make up the proposed word.
    pub tiles: Vec<Tile>,
}

impl Bet for ScrabrudoBet {
    type V = Tile;

    // TODO: Probably the naive thing will be too slow here. Use some computer science skills to
    // bring this down...
    // TODO: Preload and memoize the wordlist
    // TODO: Dictionary helper mod for the above
    fn all(state: &GameState) -> Vec<Box<Self>> {
        let f = match File::open("data/scrabble.txt") {
            Ok(file) => file,
            Err(e) => panic!("Couldn't open dictionary: {:?}", e),
        };
        // Get only those words that fit on the table.
        BufReader::new(f)
            .lines()
            .map(|l| l.unwrap())
            .filter(|l| l.len() <= state.total_num_items)
            .map(|l| Box::new(Self::from_word(l)))
            .collect()
    }

    fn smallest() -> Box<Self> {
        Box::new(Self { tiles: vec![] })
    }

    fn best_first_bet(
        state: &GameState,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> Box<Self> {
        unimplemented!();
    }

    fn perudo_prob(
        &self,
        state: &GameState,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> f64 {
        unimplemented!();
    }

    fn palafico_prob(
        &self,
        state: &GameState,
        player: Box<dyn Player<V = Self::V, B = Self>>,
    ) -> f64 {
        unimplemented!();
    }

    fn bet_prob(&self, state: &GameState, player: Box<dyn Player<V = Self::V, B = Self>>) -> f64 {
        unimplemented!();
    }
}

impl ScrabrudoBet {
    fn from_word(word: String) -> Self {
        let tiles = word
            .chars()
            .map(|c| Tile::from_char(c))
            .collect::<Vec<Tile>>();
        Self { tiles }
    }

    fn as_word(&self) -> String {
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
}
