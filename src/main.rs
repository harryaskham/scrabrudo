extern crate speculate;
extern crate rand;

use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

use speculate::speculate;

/// Anything that can make up a hand.
pub trait Holdable {
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

impl <T: Holdable> Dealer<T> for RandomDealer {
    fn deal(&self) -> T {
        T::get_random()
    }
}

#[derive(Debug)]
pub enum DieVal {
    One,
    Two,
    Three,
    Four,
    Five,
    Six
}

// Make it possible to generate random DieVals.
impl Distribution<DieVal> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> DieVal {
        match rng.gen_range(1, 7) {
            1 => DieVal::One,
            2 => DieVal::Two,
            3 => DieVal::Three,
            4 => DieVal::Four,
            5 => DieVal::Five,
            6 => DieVal::Six,
        }
    }
}

/// A single die.
pub struct Die {
    val: DieVal
}

impl Holdable for Die {
    fn get_random() -> Self {
        Die {
            val: rand::random()
        }
    }

    fn val(&self) -> DieVal {
        &self.val
    }
}

/// A single agent's hand of dice.
pub struct Hand<T: Holdable> {
    items: Vec<Box<T>>
}

impl <T: Holdable> Hand<T> {
    pub fn new(dealer: &Box<dyn Dealer<T>>, n: u32) -> Self {
        Self {
            items: dealer.deal_n(n)
        }
    }
}

fn main() {
    println!("Perudo TODO");
}

speculate! {
    describe "dealing" {
        it "deals a hand of five" {
            let dealer = Box::new(RandomDealer{});
            let hand = Hand::<Die>::new(&dealer, 5);
        }
    }
}
