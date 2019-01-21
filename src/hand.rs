/// Logic related to dealing hands.
use crate::testing;

use rand::distributions::Standard;
use rand::Rng;
use speculate::speculate;
use std::cmp::Ord;

/// Anything that can make up a hand.
pub trait Holdable {
    fn get_random() -> Self;
}

/// Anything that can deal Holdables.
pub trait Dealer<T: Holdable> {
    fn deal(&self) -> T;
    fn deal_n(&self, n: u32) -> Vec<T> {
        (0..n).into_iter().map(|_| self.deal()).collect::<Vec<T>>()
    }
}

/// A dealer that provides random cards.
pub struct RandomDealer {}

impl RandomDealer {
    pub fn new() -> Self {
        Self {}
    }
}

impl<T: Holdable> Dealer<T> for RandomDealer {
    fn deal(&self) -> T {
        T::get_random()
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Die {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
}

impl Die {
    pub fn int(&self) -> u32 {
        match &self {
            Die::One => 1,
            Die::Two => 2,
            Die::Three => 3,
            Die::Four => 4,
            Die::Five => 5,
            Die::Six => 6,
        }
    }

    pub fn all() -> Vec<Die> {
        vec![
            Die::One,
            Die::Two,
            Die::Three,
            Die::Four,
            Die::Five,
            Die::Six,
        ]
    }

    pub fn from_usize(x: usize) -> Die {
        let all = Die::all();
        all[x - 1].clone()
    }
}

// Make it possible to generate random Dies.
impl rand::distributions::Distribution<Die> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Die {
        match rng.gen_range(1, 7) {
            1 => Die::One,
            2 => Die::Two,
            3 => Die::Three,
            4 => Die::Four,
            5 => Die::Five,
            6 => Die::Six,
            _ => panic!(),
        }
    }
}

impl Holdable for Die {
    fn get_random() -> Self {
        rand::random()
    }
}

/// A single agent's hand of dice.
#[derive(Debug, Clone)]
pub struct Hand<T: Holdable> {
    pub items: Vec<T>,
}

impl<T: Holdable> Hand<T> {
    pub fn new(n: u32) -> Self {
        Self {
            // TODO: Inject dealer for testing purposes.
            items: RandomDealer::new().deal_n(n),
        }
    }
}

speculate! {
    before {
        testing::set_up();
    }

    describe "dealing" {
        it "deals a hand of five" {
            let hand = Hand::<Die>::new(5);
            assert_eq!(5, hand.items.len());
        }
    }
}
