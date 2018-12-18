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
    fn deal(&self) -> T;
    fn deal_n(&self, n: u32) -> Vec<T> {
        (0..n).into_iter().map(|_| self.deal()).collect::<Vec<T>>()
    }
}

/// A dealer that provides random cards.
pub struct RandomDealer {}

impl RandomDealer {
    fn new() -> Self {
        Self{}
    }
}

impl <T: Holdable> Dealer<T> for RandomDealer {
    fn deal(&self) -> T {
        T::get_random()
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

impl DieVal {
    fn int(&self) -> u32 {
        match &self {
            DieVal::One => 1,
            DieVal::Two => 2,
            DieVal::Three => 3,
            DieVal::Four => 4,
            DieVal::Five => 5,
            DieVal::Six => 6,
        }
    }
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
#[derive(Debug)]
pub struct Hand<T: Holdable> {
    items: Vec<T>
}

impl <T: Holdable> Hand<T> {
    pub fn new(n: u32) -> Self {
        Self {
            // TODO: Inject dealer for testing purposes.
            items: RandomDealer::new().deal_n(n)
        }
    }
}

pub struct Player {
    hand: Hand<Die>,
    // TODO: Palafico tracker
}

impl Player {
    fn new() -> Self {
        Self {
            hand: Hand::<Die>::new(5)
        }
    }
}

pub struct Game {
    players: Vec<Player>,
    turn_index: usize,
}

impl Game {
    fn new(num_players: u32) -> Self {
        let mut players: Vec<Player> = Vec::new();
        for _ in 0..num_players {
            players.push(Player::new());
        }
        Self {
            players: players,
            turn_index: 0
        }
    }
}

fn main() {
    println!("Perudo TODO");
}

speculate! {
    describe "dealing" {
        it "deals a hand of five" {
            let hand = Hand::<Die>::new(5);
            assert_eq!(5, hand.items.len()); 
        }
    }

    describe "a game" {
        it "initialises a game" {
            let _game = Game::new(6);
        }
    }
}
