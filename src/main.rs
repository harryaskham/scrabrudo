extern crate rand;
extern crate speculate;
#[macro_use]
extern crate log;
extern crate env_logger;

#[macro_use(c)]
extern crate cute;
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use speculate::speculate;
use std::collections::HashMap;

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
        Self {}
    }
}

impl<T: Holdable> Dealer<T> for RandomDealer {
    fn deal(&self) -> T {
        T::get_random()
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Hash)]
pub enum DieVal {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
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
    val: DieVal,
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
    items: Vec<T>,
}

impl<T: Holdable> Hand<T> {
    pub fn new(n: u32) -> Self {
        Self {
            // TODO: Inject dealer for testing purposes.
            items: RandomDealer::new().deal_n(n),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    id: usize,
    hand: Hand<Die>,
    // TODO: Palafico tracker
}

impl Player {
    fn new(id: usize) -> Self {
        Self {
            id: id,
            hand: Hand::<Die>::new(5),
        }
    }

    fn without_one(&self) -> Self {
        Self {
            id: self.id,
            hand: Hand::<Die>::new(self.hand.items.len() as u32 - 1),
        }
    }

    fn with_one(&self) -> Self {
        Self {
            id: self.id,
            hand: Hand::<Die>::new(self.hand.items.len() as u32 + 1),
        }
    }

    fn refresh(&self) -> Self {
        Self {
            id: self.id,
            hand: Hand::<Die>::new(self.hand.items.len() as u32),
        }
    }

    // A simple implementation of a first bet.
    // Makes the largest safe estimated bet.
    fn simple_first_bet(&self, num_dice_per_val: &HashMap<DieVal, usize>) -> Bet {
        let num_other_dice =
            num_dice_per_val.values().into_iter().sum::<usize>() - self.hand.items.len();
        // Get the number of Aces.
        let num_aces = num_dice_per_val.get(&DieVal::One).unwrap();
        // Get the most commom non-One DieVal and its quantity in the hand.
        let (most_common, quantity) = num_dice_per_val
            .into_iter()
            .filter(|x| x.0 != &DieVal::One)
            .max_by_key(|x| x.1)
            .unwrap();

        // Bet the max we believe in.
        Bet {
            value: most_common.clone(),
            quantity: (num_other_dice / 3) + num_aces + quantity,
        }
    }

    // TODO: Pluggable agent functions here for different styles.
    // TODO: Enumerate all possible outcomes and assign probability here.
    // TODO: Enforce no cheating by game introspection.
    fn play(&self, game: &Game, current_outcome: &TurnOutcome) -> TurnOutcome {
        let bet = self.simple_first_bet(&game.num_dice_per_val());
        match current_outcome {
            TurnOutcome::First => TurnOutcome::Bet(bet),
            TurnOutcome::Bet(current_bet) => {
                if bet > *current_bet {
                    return TurnOutcome::Bet(bet);
                }
                return TurnOutcome::Perudo;
            }
            TurnOutcome::Perudo => panic!(),
        }
    }
}

// TODO: Implement ordering, increment here for easy bet generation.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Bet {
    value: DieVal,
    quantity: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TurnOutcome {
    First,
    Bet(Bet),
    Perudo,
}

pub struct Game {
    players: Vec<Player>,
}

impl Game {
    fn new(num_players: usize) -> Self {
        let mut players: Vec<Player> = Vec::new();
        for id in 0..num_players {
            players.push(Player::new(id));
        }
        Self { players: players }
    }

    fn num_players(&self) -> usize {
        self.players.len()
    }

    fn num_dice_per_val(&self) -> HashMap<DieVal, usize> {
        c! { val.clone() => self.num_dice(&val), for val in DieVal::all().into_iter() }
    }

    fn num_dice(&self, val: &DieVal) -> usize {
        (&self.players)
            .into_iter()
            .map(|p| &p.hand.items)
            .flatten()
            .filter(|d| &d.val() == val)
            .count()
    }

    fn is_correct(&self, bet: &Bet) -> bool {
        (bet.value == DieVal::One && bet.quantity <= self.num_dice(&DieVal::One))
            || (bet.value != DieVal::One
                && bet.quantity <= (self.num_dice(&DieVal::One) + self.num_dice(&bet.value)))
    }

    fn num_dice_per_player(&self) -> Vec<usize> {
        self.players
            .clone()
            .into_iter()
            .map(|p| p.hand.items.len())
            .collect()
    }

    fn run(&mut self) {
        info!("Game commencing");
        let mut current_index: usize = 0;
        let mut current_outcome = TurnOutcome::First;
        // TODO: Remove hack via an Option.
        let mut last_bet = Bet {
            value: DieVal::One,
            quantity: 0,
        };
        loop {
            // TODO: Include historic bets in the context given to the player.
            let player = &self.players[current_index];
            current_outcome = player.play(self, &current_outcome);
            match &current_outcome {
                TurnOutcome::Bet(bet) => {
                    info!("Player {} bets {:?}", player.id, bet);
                    last_bet = bet.clone();
                    current_index = (current_index + 1) % self.num_players();
                }
                TurnOutcome::Perudo => {
                    info!("Player {} calls Perudo", player.id);
                    let loser_index: usize;
                    if self.is_correct(&last_bet) {
                        info!("Player {} is incorrect, the bet was good!", player.id);
                        loser_index = current_index;
                    } else {
                        info!("Player {} is correct, the bet was bad!", player.id);
                        loser_index = (current_index + self.num_players() - 1) % self.num_players();
                    };
                    match self.end_turn(loser_index) {
                        Some(i) => {
                            current_index = i;
                            current_outcome = TurnOutcome::First;
                        }
                        None => {
                            break;
                        }
                    };
                }
                TurnOutcome::First => panic!(),
            };
        }
    }

    // Ends the turn and returns the index of the next player.
    fn end_turn(&mut self, loser_index: usize) -> Option<usize> {
        let loser = &self.players[loser_index];
        if loser.hand.items.len() == 1 {
            if self.players.len() > 2 {
                // This player is disqualified.
                info!("Player {} is disqualified", loser.id);
                self.players.remove(loser_index);
                return Some((loser_index % self.num_players()) as usize);
            } else {
                info!("All players are eliminated.");
                return None;
            }
        } else {
            // Refresh all players, loser loses an item.
            self.players = self
                .players
                .clone()
                .into_iter()
                .enumerate()
                .map(|(i, p)| {
                    if i == loser_index {
                        p.without_one()
                    } else {
                        p.refresh()
                    }
                })
                .collect();
            info!("Player {} loses a die, now has {}.",
                  self.players[loser_index].id,
                  self.players[loser_index].hand.items.len());
            return Some(loser_index);
        }
    }
}

fn main() {
    env_logger::init();
    info!("Perudo 0.1");
    let mut game = Game::new(6);
    game.run();
}

speculate! {
    before {
        env_logger::try_init();
    }

    describe "dealing" {
        it "deals a hand of five" {
            let hand = Hand::<Die>::new(5);
            assert_eq!(5, hand.items.len());
        }
    }

    describe "a game" {
        it "runs to completion" {
            let mut game = Game::new(6);
            game.run();
        }

        it "runs an expected game setup" {
            let mut game = Game {
                players: vec![
                    Player {
                        id: 0,
                        hand: Hand::<Die> {
                            items: vec![
                                Die{ val: DieVal::One },
                                Die{ val: DieVal::Two },
                                Die{ val: DieVal::Two },
                                Die{ val: DieVal::Five },
                                Die{ val: DieVal::Six }
                            ],
                        },
                    },
                    Player {
                        id: 1,
                        hand: Hand::<Die> {
                            items: vec![
                                Die{ val: DieVal::One },
                                Die{ val: DieVal::One },
                                Die{ val: DieVal::Six },
                                Die{ val: DieVal::Six },
                                Die{ val: DieVal::Three }
                            ],
                        },
                    },
                    Player {
                        id: 2,
                        hand: Hand::<Die> {
                            items: vec![
                                Die{ val: DieVal::Five },
                                Die{ val: DieVal::Five },
                                Die{ val: DieVal::Five },
                                Die{ val: DieVal::Two },
                                Die{ val: DieVal::Three }
                            ],
                        },
                    },
                ],
            };
            game.run();
        }

    }
}
