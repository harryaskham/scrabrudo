extern crate rand;
extern crate speculate;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use] extern crate itertools;

#[macro_use(c)]
extern crate cute;
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use speculate::speculate;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::cmp::Ord;
use std::cmp::Ordering;
use std::env;
use std::io;

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

    pub fn from_usize(x: usize) -> DieVal {
        let all = DieVal::all();
        all[x - 1].clone()
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
    human: bool,
    // TODO: Palafico tracker
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {:?}", self.id, (&self.hand.items)
            .into_iter()
            .map(|d| d.val.int())
            .collect::<Vec<u32>>())
    }
}

impl Player {
    fn new(id: usize, human: bool) -> Self {
        Self {
            id: id,
            human: human,
            hand: Hand::<Die>::new(5),
        }
    }

    fn without_one(&self) -> Self {
        Self {
            id: self.id,
            human: self.human,
            hand: Hand::<Die>::new(self.hand.items.len() as u32 - 1),
        }
    }

    fn with_one(&self) -> Self {
        Self {
            id: self.id,
            human: self.human,
            hand: Hand::<Die>::new(self.hand.items.len() as u32 + 1),
        }
    }

    fn refresh(&self) -> Self {
        Self {
            id: self.id,
            human: self.human,
            hand: Hand::<Die>::new(self.hand.items.len() as u32),
        }
    }

    fn num_dice(&self, val: DieVal) -> usize {
        (&self.hand.items)
            .into_iter()
            .filter(|d| d.val() == val)
            .count()
    }

    fn num_dice_per_val(&self) -> HashMap<DieVal, usize> {
        c! { val.clone() => self.num_dice(val), for val in DieVal::all().into_iter() }
    }

    // A simple implementation of a first bet.
    // Makes the largest safe estimated bet.
    fn simple_first_bet(&self, total_num_dice: usize) -> Bet {
        let num_other_dice = total_num_dice - self.hand.items.len();
        let num_aces = self.num_dice(DieVal::One);
        // Get the most commom non-One DieVal and its quantity in the hand.
        let (most_common, quantity) = self.num_dice_per_val()
            .into_iter()
            .filter(|x| x.0 != DieVal::One)
            .max_by_key(|x| x.1)
            .unwrap();
        debug!("Making bet based on {} aces, {} {:?}s in the hand, and {} other dice",
               num_aces, quantity, most_common, num_other_dice);

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
        // TODO: More elegant way of implementing multiple play strategies.
        if self.human {
            return self.human_play(game, current_outcome);
        }

        let bet = self.simple_first_bet(game.total_num_dice());
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

    // TODO: Make this a play function on some trait.
    fn human_play(&self, game: &Game, current_outcome: &TurnOutcome) -> TurnOutcome {
        loop {
            info!("Dice left: {:?} ({})", game.num_dice_per_player(), game.total_num_dice());
            info!("Hand for Player {})", self);
            match current_outcome {
                TurnOutcome::First => info!("Enter bet (2.6=two sixes):"),
                TurnOutcome::Bet(_) => info!("Enter bet (2.6=two sixes, p=perudo):"),
                TurnOutcome::Perudo => panic!(),
            };

            let mut line = String::new();
            io::stdin().read_line(&mut line).expect("Failed to read input");
            let line = line.trim();

            if line == "p" {
                return TurnOutcome::Perudo;
            }

            // Parse input, repeat on error.
            // TODO: Helpers for the below.
            let mut split = line.split(".");
            let quantity = match split.next() {
                Some(q) => match q.parse::<usize>() {
                    Ok(q) => q,
                    Err(e) => {
                        info!("{}", e);
                        continue;
                    }
                },
                None => continue
            };

            let value = match split.next() {
                Some(v) => match v.parse::<usize>() {
                    Ok(v) => v,
                    Err(e) => {
                        info!("{}", e);
                        continue;
                    }
                },
                None => continue
            };

            // Either return a valid bet or take input again.
            let bet = Bet {
                value: DieVal::from_usize(value),
                quantity: quantity
            };
            return match current_outcome {
                TurnOutcome::First => TurnOutcome::Bet(bet),
                TurnOutcome::Bet(current_bet) => {
                    if bet > *current_bet {
                        return TurnOutcome::Bet(bet);
                    } else {
                        continue;
                    }
                },
                TurnOutcome::Perudo => panic!(),
            };
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct Bet {
    value: DieVal,
    quantity: usize,
}

impl Bet {
    // Get all possible bets above the one given.
    fn all_above(&self, num_dice: usize) -> Vec<Self> {
        // Generate all bets and filter down to only those which are greater than the one given.
        iproduct!(DieVal::all().into_iter(), 1..=num_dice)
            .map(|(value, quantity)| Bet {
                value: value,
                quantity: quantity,
            })
            .filter(|b| b > self)
            .collect::<Vec<Bet>>()
    }

    fn prob(self, num_dice: usize) -> f64 {
        0.00
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
        } else if (self.value == other.value && self.quantity > other.quantity) ||
            (self.value > other.value && self.quantity >= other.quantity) {
            // If we've increased the die quantity only then the bet is larger.
            Ordering::Greater
        } else if (self.value == other.value && self.quantity == other.quantity) {
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

#[derive(Debug, Clone, PartialEq)]
pub enum TurnOutcome {
    First,
    Bet(Bet),
    Perudo,
}

pub struct Game {
    players: Vec<Player>,
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Hands: {:?}", (&self.players)
            .into_iter()
            .map(|p| format!("{}", p))
            .collect::<Vec<String>>()
            .join("\n"))
    }
}

impl Game {
    fn new(num_players: usize, human_indices: HashSet<usize>) -> Self {
        let mut players: Vec<Player> = Vec::new();
        for id in 0..num_players {
            let human = human_indices.contains(&id);
            players.push(Player::new(id, human));
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

    // Gets the actual number of dice around the table, allowing for wildcards.
    fn num_logical_dice(&self, val: &DieVal) -> usize {
        if val == &DieVal::One {
            self.num_dice(&DieVal::One)
        } else {
            self.num_dice(&DieVal::One) + self.num_dice(val)
        }
    }

    fn is_correct(&self, bet: &Bet) -> bool {
        let max_correct_bet = Bet {
            value: bet.value.clone(),
            quantity: self.num_logical_dice(&bet.value),
        };
        debug!("Maximum allowable bet is {}", max_correct_bet);
        bet <= &max_correct_bet
    }

    fn num_dice_per_player(&self) -> Vec<usize> {
        self.players
            .clone()
            .into_iter()
            .map(|p| p.hand.items.len())
            .collect()
    }

    fn total_num_dice(&self) -> usize {
        self.num_dice_per_player().iter().sum()
    }

    fn run(&mut self) {
        let mut current_index: usize = 0;
        let mut current_outcome = TurnOutcome::First;
        // TODO: Remove hack via an Option.
        let mut last_bet = Bet {
            value: DieVal::One,
            quantity: 0,
        };
        loop {
            // TODO: Include historic bets in the context given to the player.
            debug!("{}", self);  // Print the game state.
            let player = &self.players[current_index];
            current_outcome = player.play(self, &current_outcome);
            match &current_outcome {
                TurnOutcome::Bet(bet) => {
                    info!("Player {} bets {}", player.id, bet);
                    last_bet = bet.clone();
                    current_index = (current_index + 1) % self.num_players();
                }
                TurnOutcome::Perudo => {
                    info!("Player {} calls Perudo", player.id);
                    let loser_index: usize;
                    let actual_amount = self.num_logical_dice(&last_bet.value);
                    if self.is_correct(&last_bet) {
                        info!("Player {} is incorrect, there were {} {:?}s", player.id, actual_amount, last_bet.value);
                        loser_index = current_index;
                    } else {
                        info!("Player {} is correct, there were {} {:?}s", player.id, actual_amount, last_bet.value);
                        loser_index = (current_index + self.num_players() - 1) % self.num_players();
                    };
                    match self.end_turn(loser_index) {
                        Some(i) => {
                            current_index = i;
                            current_outcome = TurnOutcome::First;
                        }
                        None => {
                            info!("Player {} wins!", self.players[0].id);
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
            info!("Player {} is disqualified", loser.id);
            self.players.remove(loser_index);

            if self.players.len() > 1 {
                return Some((loser_index % self.num_players()) as usize);
            } else {
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
            info!("Player {} loses a die, now has {}",
                  self.players[loser_index].id,
                  self.players[loser_index].hand.items.len());
            return Some(loser_index);
        }
    }
}

fn main() {
    pretty_env_logger::init();
    let args: Vec<String> = env::args().collect();

    info!("Perudo 0.1");
    if args.len() < 2 {
        info!("Please supply number of players");
        return
    }

    let num_players = args[1].parse::<usize>().unwrap();
    let mut human_indices = HashSet::new();
    human_indices.insert(0);
    let mut game = Game::new(num_players, human_indices);
    game.run();
}

speculate! {
    before {
        pretty_env_logger::try_init();
    }

    describe "dealing" {
        it "deals a hand of five" {
            let hand = Hand::<Die>::new(5);
            assert_eq!(5, hand.items.len());
        }
    }

    describe "bets" {
        fn bet(v: DieVal, q: usize) -> Bet {
            Bet {
                value: v,
                quantity: q,
            }
        }

        it "orders bets correctly" {
            let bet_1 = bet(DieVal::Two, 1);
            let bet_2 = bet(DieVal::Two, 2);
            let bet_3 = bet(DieVal::Two, 6);
            let bet_4 = bet(DieVal::Three, 6);
            let bet_5 = bet(DieVal::Three, 7);
            let bet_6 = bet(DieVal::Five, 7);
            let bet_7 = bet(DieVal::Six, 7);
            let bet_8 = bet(DieVal::Six, 8);
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
            let original = Bet {
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
                original.all_above(2));
        }
    }

    describe "a game" {
        it "runs to completion" {
            let mut game = Game::new(6, HashSet::new());
            game.run();
        }

        it "runs an expected game setup" {
            let mut game = Game {
                players: vec![
                    Player {
                        id: 0,
                        human: false,
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
                        human: false,
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
                        human: false,
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
