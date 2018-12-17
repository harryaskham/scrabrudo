extern crate speculate;
extern crate rand;

use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

use speculate::speculate;

/// Anything that can make up a hand.
pub trait Holdable {
    fn with_val(val: DieVal) -> Self;
    fn get_random() -> Self;
    fn val(&self) -> DieVal;
}

/// Anything that can deal Holdables.
pub trait Dealer<T: Holdable> {
    fn deal(&mut self) -> T;
    fn deal_n(&mut self, n: u32) -> Vec<T> {
        (0..n).into_iter().map(|_| self.deal()).collect::<Vec<T>>()
    }
}

/// A dealer that provides random cards.
pub struct RandomDealer {}

impl Dealer<Die> for RandomDealer {
    fn deal(&mut self) -> Die {
        Die::get_random()
    }
}

/// A dealer that deals sequential cards.
/// Useful for testing.
pub struct SequentialDealer {
    next: DieVal
}

impl SequentialDealer {
    fn new() -> Self {
        Self{next: DieVal::Six}
    }
}

impl Dealer<Die> for SequentialDealer {
    fn deal(&mut self) -> Die {
        self.next = match self.next {
            DieVal::One => DieVal::Two,
            DieVal::Two => DieVal::Three,
            DieVal::Three => DieVal::Four,
            DieVal::Four => DieVal::Five,
            DieVal::Five => DieVal::Six,
            DieVal::Six => DieVal::One,
        };
        Die::with_val(self.next.clone())
    }
}

#[derive(Debug, Clone, PartialEq)]
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
            _ => DieVal::One,
        }
    }
}

/// A single die.
pub struct Die {
    val: DieVal
}

impl Holdable for Die {
    fn with_val(val: DieVal) -> Self {
        Self { val }
    }

    fn get_random() -> Self {
        Self {
            val: rand::random()
        }
    }

    fn val(&self) -> DieVal {
        self.val.clone()
    }
}

/// A single agent's hand of dice.
pub struct Hand<T: Holdable> {
    items: Vec<T>
}

impl <T: Holdable> Hand<T> {
    pub fn new(mut dealer: Box<dyn Dealer<T>>, n: u32) -> Self {
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
            let dealer = Box::new(SequentialDealer::new());
            let hand = Hand::<Die>::new(dealer, 5);
            assert_eq!(5, hand.items.len()); 
            assert_eq!(vec![DieVal::One, DieVal::Two, DieVal::Three, DieVal::Four, DieVal::Five],
                       hand.items.into_iter().map(|i| i.val()).collect::<Vec<DieVal>>());
        }

        it "deals a random hand" {
            let dealer = Box::new(RandomDealer{});
            let hand = Hand::<Die>::new(dealer, 5);
            assert_eq!(5, hand.items.len()); 
        }
    }
}
