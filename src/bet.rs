/// Bet definitions and related logic.

use crate::player::Player;

use probability::prelude::*;
use std::cmp::Ord;
use std::cmp::Ordering;
use std::fmt;

use crate::hand::*;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct Bet {
    // TODO: Make private again and enforce immutability via with/without functions.
    pub value: DieVal,
    pub quantity: usize,
}

impl Bet {
    // Generate all possible bets.
    pub fn all(num_dice: usize) -> Vec<Self> {
        iproduct!(DieVal::all().into_iter(), 1..=num_dice)
            .map(|(value, quantity)| Bet {
                value: value,
                quantity: quantity,
            })
            .collect::<Vec<Bet>>()
    }

    pub fn all_without_ones(num_dice: usize) -> Vec<Self> {
        Bet::all(num_dice)
            .into_iter()
            .filter(|b| b.value != DieVal::One)
            .collect::<Vec<Bet>>()
    }

    // Get all possible bets above the one given.
    pub fn all_above(&self, num_dice: usize) -> Vec<Self> {
        // Generate all bets and filter down to only those which are greater than the one given.
        Bet::all(num_dice)
            .into_iter()
            .filter(|b| b > self)
            .collect::<Vec<Bet>>()
    }

    // Gets the probability that this bet is incorrect as far as the given player is concerned.
    pub fn perudo_prob(&self, total_num_dice: usize, player: &Player) -> f64 {
        1.0 - self.prob(total_num_dice, player) 
    }

    // Gets the probability that this bet is exactly correct as far as the given player is
    // concerned.
    pub fn palafico_prob(&self, total_num_dice: usize, player: &Player) -> f64 {
        let guaranteed_quantity = player.num_logical_dice(self.value.clone());
        if guaranteed_quantity > self.quantity {
            return 0.0;
        }

        let trial_p: f64 = if self.value == DieVal::One {
            1.0 / 6.0
        } else {
            1.0 / 3.0
        };
        let num_other_dice = total_num_dice - player.hand.items.len();
        // This is a single Binomial trial - what's the probability of finding the rest of the dice
        // in the remaining dice.
        Binomial::new(num_other_dice, trial_p).mass(self.quantity - guaranteed_quantity)
    }

    // Get the probability of the bet being correct.
    // This is akin to the mass of this bet, plus all those with the same value and higher
    // quantity.
    // We also take into account only the other dice and count those we have in the given hand as
    // guaranteed.
    pub fn prob(&self, total_num_dice: usize, player: &Player) -> f64 {
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
        let num_other_dice = total_num_dice - player.hand.items.len();
        ((self.quantity - guaranteed_quantity)..=num_other_dice)
            .map(|q| Binomial::new(num_other_dice, trial_p).mass(q))
            .sum::<f64>()
    }
}

impl fmt::Display for Bet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {:?}s", self.quantity, self.value)
    }
}

impl Ord for Bet {
    fn cmp(&self, other: &Bet) -> Ordering {
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

impl PartialOrd for Bet {
    fn partial_cmp(&self, other: &Bet) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

