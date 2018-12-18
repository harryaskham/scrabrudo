extern crate speculate;
extern crate rand;
#[macro_use] extern crate itertools;

use itertools::Itertools;

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

#[derive(Debug, Clone, PartialEq, PartialOrd)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

    fn without_one(&self) -> Self {
        Self {
            hand: Hand::<Die>::new(self.hand.items.len() as u32 - 1)
        }
    }

    fn with_one(&self) -> Self {
        Self {
            hand: Hand::<Die>::new(self.hand.items.len() as u32 + 1)
        }
    }

    fn refresh(&self) -> Self {
        Self {
            hand: Hand::<Die>::new(self.hand.items.len() as u32)
        }
    }

    // A simple implementation of a first bet.
    // Makes the largest safe estimated bet.
    fn simple_first_bet(&self, num_dice: usize) -> Bet {
        let num_other_dice = num_dice - self.hand.items.len();
        // Find the most common item in the hand.
        // Add the other amount over 3 plus the ones in the hand.
        //self.hand.items.into_iter().fold
        Bet {
            value: DieVal::Six,
            quantity: 6
        }
    }

    // TODO: Pluggable agent functions here for different styles.
    // TODO: Enumerate all possible outcomes and assign probability here.
    // TODO: Enforce no cheating by game introspection.
    fn play(&self,
            game: &Game,
            current_outcome: &TurnOutcome) -> TurnOutcome {
        let num_dice = game.num_dice_per_player().into_iter().sum();
        let bet = self.simple_first_bet(num_dice);
        match current_outcome {
            TurnOutcome::First => TurnOutcome::Bet(bet),
            TurnOutcome::Bet(current_bet) => {
                if bet > *current_bet {
                    return TurnOutcome::Bet(bet);
                }
                return TurnOutcome::Perudo;
            },
            TurnOutcome::Perudo => panic!(),
        }
    }
}

// TODO: Implement ordering, increment here for easy bet generation.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Bet {
    value: DieVal,
    quantity: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TurnOutcome {
    First,
    Bet(Bet),
    Perudo
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

    fn num_players(&self) -> usize {
        self.players.len()
    }

    fn is_correct(&self, bet: &Bet) -> bool {
        false  // TODO
    }

    fn num_dice_per_player(&self) -> Vec<usize> {
        self.players.clone()
            .into_iter()
            .map(|p| p.hand.items.len())
            .collect()
    }

    fn run_turn(&mut self) {
        let mut current_index = self.turn_index.clone();
        let mut current_outcome = TurnOutcome::First;
        // TODO: Remove hack via an Option.
        let mut last_bet = Bet {
            value: DieVal::One,
            quantity: 0,
        };
        let mut last_player: &Player;
        loop {
            // TODO: Include historic bets in the context given to the player.
            let player = &self.players[current_index];
            current_outcome = player.play(self, &current_outcome);
            match &current_outcome {
                TurnOutcome::Bet(bet) => {
                    last_bet = bet.clone();
                    last_player = &player;
                    current_index = (current_index + 1) % self.num_players();
                    bet
                },
                TurnOutcome::Perudo => {
                    if self.is_correct(&last_bet) {
                        self.end_turn(current_index);
                    } else {
                        self.end_turn((current_index - 1) % self.num_players());
                    }
                    break;
                }
                TurnOutcome::First => panic!(),
            };
        }
    }

    fn end_turn(&mut self, loser_index: usize) {
        let loser = &self.players[loser_index];
        if loser.hand.items.len() == 1 {
            if self.players.len() > 2 {
                // This player is disqualified.
                self.players.remove(loser_index);
                self.turn_index = loser_index % self.num_players()
            } else {
                // End of game!
            }
        } else {
            // Refresh all players, loser loses an item.
            self.players = self.players.clone().into_iter()
                .enumerate()
                .map(|(i, p)| if i == loser_index { p.without_one() } else { p.refresh() })
                .collect();
        }
    }
}

fn main() {
    let mut game = Game::new(6);
}

speculate! {
    describe "dealing" {
        it "deals a hand of five" {
            let hand = Hand::<Die>::new(5);
            assert_eq!(5, hand.items.len()); 
        }
    }

    describe "a game" {
        it "runs a turn" {
            let mut game = Game::new(6);
            game.run_turn();
        }
    }
}
