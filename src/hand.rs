/// Logic related to dealing hands.
use crate::testing;

use rand::distributions::Standard;
use rand::Rng;
use speculate::speculate;
use std::cmp::Ord;

/// Anything that can make up a hand.
pub trait Holdable {
    fn with_val(val: DieVal) -> Self;
    fn get_random() -> Self;
    fn val(&self) -> DieVal;
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

impl <T: Holdable> Dealer<T> for RandomDealer {
    fn deal(&self) -> T {
        T::get_random()
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum DieVal {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
}

impl DieVal {
    pub fn int(&self) -> u32 {
        match &self {
            DieVal::One => 1,
            DieVal::Two => 2,
            DieVal::Three => 3,
            DieVal::Four => 4,
            DieVal::Five => 5,
            DieVal::Six => 6,
        }
    }

    pub fn all() -> Vec<DieVal> {
        vec![
            DieVal::One,
            DieVal::Two,
            DieVal::Three,
            DieVal::Four,
            DieVal::Five,
            DieVal::Six,
        ]
    }

    pub fn from_usize(x: usize) -> DieVal {
        let all = DieVal::all();
        all[x - 1].clone()
    }
}

// Make it possible to generate random DieVals.
impl rand::distributions::Distribution<DieVal> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> DieVal {
        match rng.gen_range(1, 7) {
            1 => DieVal::One,
            2 => DieVal::Two,
            3 => DieVal::Three,
            4 => DieVal::Four,
            5 => DieVal::Five,
            6 => DieVal::Six,
            _ => DieVal::One,
        }
    }
}

/// A single die.
#[derive(Debug, Clone)]
pub struct Die {
    pub val: DieVal,
}

impl Holdable for Die {
    fn with_val(val: DieVal) -> Self {
        Self { val }
    }

    fn get_random() -> Self {
        Self {
            val: rand::random(),
        }
    }

    fn val(&self) -> DieVal {
        self.val.clone()
    }
}

/// A single agent's hand of dice.
#[derive(Debug, Clone)]
pub struct Hand<T: Holdable> {
    // TODO: Make private again and enforce immutability via with/without functions.
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
