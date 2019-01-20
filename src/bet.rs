use crate::game::*;
use crate::hand::*;
/// Bet definitions and related logic.
use crate::player::*;
use crate::testing;

use probability::prelude::*;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;
use speculate::speculate;
use std::cmp::Ord;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt;

/// Trait implemented by any type of bet.
pub trait Bet: Ord + Clone {
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
    fn best_first_bet(state: &GameState, player: &Player) -> Box<Self>;

    /// Get the probability of this bet being correct.
    /// TODO: Need to make Player itself a boxed trait, because this is all still Perudo-specific.
    /// TODO: Need to make ProbVarient itself a boxed trait enum, same reason.
    fn prob(&self, state: &GameState, variant: ProbVariant, player: &Player) -> f64;
}

/// The different types of Bet one can make in Perudo.
pub enum ProbVariant {
    Bet,
    Perudo,
    Palafico,
}

// TODO: Rename PerudoBet and refactor
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct PerudoBet {
    pub value: DieVal,
    pub quantity: usize,
}

impl Bet for PerudoBet {
    fn all(state: &GameState) -> Vec<Box<Self>> {
        iproduct!(DieVal::all().into_iter(), 1..=state.num_items)
            .map(|(value, quantity)| {
                Box::new(PerudoBet {
                    value: value,
                    quantity: quantity,
                })
            })
            .collect::<Vec<Box<PerudoBet>>>()
    }

    fn prob(&self, state: &GameState, variant: ProbVariant, player: &Player) -> f64 {
        match variant {
            ProbVariant::Bet => self.bet_prob(state, player),
            ProbVariant::Perudo => self.perudo_prob(state, player),
            ProbVariant::Palafico => self.palafico_prob(state, player),
        }
    }

    fn smallest() -> Box<Self> {
        Box::new(Self {
            quantity: 0,
            value: DieVal::Two,
        })
    }

    /// TODO: Better than random choice from equally likely bets.
    fn best_first_bet(state: &GameState, player: &Player) -> Box<Self> {
        let bets = Self::first_bets(state, player);
        let max_prob = bets[bets.len() - 1].prob(state, ProbVariant::Bet, player);
        let best_bets = bets
            .into_iter()
            .filter(|b| b.prob(state, ProbVariant::Bet, player) == max_prob)
            .collect::<Vec<Box<Self>>>();
        let mut rng = thread_rng();
        best_bets.choose(&mut rng).unwrap().clone()
    }
}

impl PerudoBet {
    /// Get the allowed first bets - everything but ones.
    /// Bets are ordered by their probability of occuring.
    fn first_bets(state: &GameState, player: &Player) -> Vec<Box<Self>> {
        Self::ordered_bets(state, player)
            .into_iter()
            .filter(|b| b.value != DieVal::One)
            .collect::<Vec<Box<Self>>>()
    }

    /// Gets all bets ordered by probability.
    fn ordered_bets(state: &GameState, player: &Player) -> Vec<Box<Self>> {
        let mut bets = Self::all(state)
            .into_iter()
            // TODO: Remove awful hack to get around lack of Ord on f64 and therefore no sort().
            .map(|b| {
                (
                    (100000.0 * b.prob(state, ProbVariant::Bet, player)) as u64,
                    b,
                )
            })
            .collect::<Vec<(u64, Box<Self>)>>();
        bets.sort_by(|a, b| a.0.cmp(&b.0));
        bets.into_iter().map(|x| x.1).collect::<Vec<Box<Self>>>()
    }

    /// All the valid bets without aces, for first-turn purposes.
    pub fn all_without_ones(state: &GameState) -> Vec<Box<Self>> {
        PerudoBet::all(state)
            .into_iter()
            .filter(|b| b.value != DieVal::One)
            .collect::<Vec<Box<PerudoBet>>>()
    }

    /// Gets the probability that this bet is incorrect as far as the given player is concerned.
    pub fn perudo_prob(&self, state: &GameState, player: &Player) -> f64 {
        1.0 - self.bet_prob(state, player)
    }

    /// Gets the probability that this bet is exactly correct as far as the given player is
    /// concerned.
    pub fn palafico_prob(&self, state: &GameState, player: &Player) -> f64 {
        let guaranteed_quantity = player.num_logical_dice(self.value.clone());
        if guaranteed_quantity > self.quantity {
            return 0.0;
        }

        let trial_p: f64 = if self.value == DieVal::One {
            1.0 / 6.0
        } else {
            1.0 / 3.0
        };
        let num_other_dice = state.num_items - player.hand.items.len();
        // This is a single Binomial trial - what's the probability of finding the rest of the dice
        // in the remaining dice.
        // TODO: This occasionally crashes in the mass() func, possibly due to overflow.
        Binomial::new(num_other_dice, trial_p).mass(self.quantity - guaranteed_quantity)
    }

    /// Get the probability of the bet being correct.
    /// This is akin to the mass of this bet, plus all those with the same value and higher
    /// quantity.
    /// We also take into account only the other dice and count those we have in the given hand as
    /// guaranteed.
    pub fn bet_prob(&self, state: &GameState, player: &Player) -> f64 {
        // If we have the bet in-hand, then we're good; otherwise we only have to look for the diff
        // in the other probabilities.
        let guaranteed_quantity = player.num_logical_dice(self.value.clone());
        if self.quantity <= guaranteed_quantity {
            return 1.0;
        }

        // TODO: Reframe the below as 1 minus the CDF of up to the bet.
        // Since we say the bet is correct if there are really n or higher.
        // We want 1 minus the probability there are less than n.
        // So that's 1 - cdf(n - 1)
        let trial_p: f64 = if self.value == DieVal::One {
            1.0 / 6.0
        } else {
            1.0 / 3.0
        };
        let num_other_dice = state.num_items - player.hand.items.len();
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
        if self.value == DieVal::One && other.value == DieVal::One {
            // If both are ace, then just compare the values.
            self.quantity.cmp(&other.quantity)
        } else if self.value == DieVal::One {
            // If this is ace, compare its double.
            // We don't +1 here as we want 1x1 to be less than 3x2, not equal.
            // We also do not define equality here in order to enforce unidirectionality of
            // ace-lifting.
            if self.quantity * 2 >= other.quantity {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        } else if other.value == DieVal::One {
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

speculate! {
    before {
        testing::set_up();
    }

    describe "perudo bets" {
        fn bet(v: DieVal, q: usize) -> Box<PerudoBet> {
            Box::new(PerudoBet {
                value: v,
                quantity: q,
            })
        }

        it "orders bets correctly" {
            let bet_1 = bet(DieVal::Two, 1);
            let bet_2 = bet(DieVal::Two, 2);
            let bet_3 = bet(DieVal::Two, 3);
            let bet_4 = bet(DieVal::Three, 3);
            let bet_5 = bet(DieVal::Three, 4);
            let bet_6 = bet(DieVal::Two, 5);
            let bet_7 = bet(DieVal::Two, 6);
            let bet_8 = bet(DieVal::Three, 8);
            let bet_9 = bet(DieVal::Six, 10);

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
            let bet_1 = bet(DieVal::Two, 1);
            let bet_2 = bet(DieVal::One, 1);
            let bet_3 = bet(DieVal::Two, 3);
            let bet_4 = bet(DieVal::Two, 4);
            let bet_5 = bet(DieVal::One, 2);
            let bet_6 = bet(DieVal::One, 3);
            let bet_7 = bet(DieVal::Five, 7);
            let bet_8 = bet(DieVal::One, 4);
            let bet_9 = bet(DieVal::Six, 9);

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
                value: DieVal::Two,
                quantity: 1,
            };
            assert_eq!(
                vec![
                    bet(DieVal::One, 1),
                    bet(DieVal::One, 2),
                    bet(DieVal::Two, 2),
                    bet(DieVal::Three, 1),
                    bet(DieVal::Three, 2),
                    bet(DieVal::Four, 1),
                    bet(DieVal::Four, 2),
                    bet(DieVal::Five, 1),
                    bet(DieVal::Five, 2),
                    bet(DieVal::Six, 1),
                    bet(DieVal::Six, 2),
                ],
                original.all_above(&GameState{num_items: 2}));
        }

        fn approx(x: f64, y: f64) {
            if (x - y).abs() > 0.001 {
                panic!("{} != {}", x, y);
            }
        }

        it "computes probability for bets" {
            // Create a player with a few of each.
            let player = Player {
                id: 0,
                human: false,
                hand: Hand::<Die> {
                    items: vec![
                        Die{ val: DieVal::One },
                        Die{ val: DieVal::Two },
                        Die{ val: DieVal::Three },
                        Die{ val: DieVal::Four },
                        Die{ val: DieVal::Five }
                    ],
                },
            };

            let state = &GameState{num_items: 6};

            // Bets on Ones, given one in the hand.
            approx(1.0, bet(DieVal::One, 0).prob(state, ProbVariant::Bet, &player));
            approx(1.0, bet(DieVal::One, 1).prob(state, ProbVariant::Bet, &player));
            approx(1.0 / 6.0, bet(DieVal::One, 2).prob(state, ProbVariant::Bet, &player));

            // We have two 2s in the hand already.
            approx(1.0, bet(DieVal::Two, 0).prob(state, ProbVariant::Bet, &player));
            approx(1.0, bet(DieVal::Two, 1).prob(state, ProbVariant::Bet, &player));
            approx(1.0, bet(DieVal::Two, 2).prob(state, ProbVariant::Bet, &player));
            approx(1.0 / 3.0, bet(DieVal::Two, 3).prob(state, ProbVariant::Bet, &player));

            // TODO: More tests for the prob-calcs.
        }

        it "generates the most likely bet" {
            let player = Player {
                id: 0,
                human: false,
                hand: Hand::<Die> {
                    items: vec![
                        Die{ val: DieVal::Six },
                        Die{ val: DieVal::Six },
                        Die{ val: DieVal::Six },
                        Die{ val: DieVal::Six },
                        Die{ val: DieVal::Six }
                    ],
                },
            };
            let total_num_dice = 5;
            let opponent_bet = &PerudoBet {
                quantity: 4,
                value: DieVal::Six,
            };
            let best_outcome_above = player.best_outcome_above(opponent_bet, total_num_dice);
            assert_eq!(best_outcome_above, TurnOutcome::Bet(PerudoBet {
                quantity: 5,
                value: DieVal::Six,
            }));
        }

        it "calls palafico with no other option" {
            let player = Player {
                id: 0,
                human: false,
                hand: Hand::<Die> {
                    items: vec![
                        Die{ val: DieVal::Six },
                    ],
                },
            };
            let total_num_dice = 2;
            let opponent_bet = &PerudoBet {
                quantity: 1,
                value: DieVal::Six,
            };
            let best_outcome_above = player.best_outcome_above(opponent_bet, total_num_dice);
            assert_eq!(best_outcome_above, TurnOutcome::Palafico);
        }
    }
}
