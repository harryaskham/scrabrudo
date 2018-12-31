extern crate rand;
extern crate speculate;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate itertools;
extern crate probability;
#[macro_use]
extern crate approx;

#[macro_use(c)]
extern crate cute;
use probability::distribution::Distribution;
use probability::prelude::*;
use rand::distributions::Standard;
use rand::Rng;
use rand::thread_rng;
use rand::seq::SliceRandom;
use speculate::speculate;
use std::cmp::min;
use std::cmp::Ord;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io;
use std::rc::Rc;

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
}

impl PartialEq for Player {
    fn eq(&self, other: &Player) -> bool {
        // TODO: Better equality for Players.
        self.id == other.id
    }
}

impl Eq for Player {}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}: {:?}",
            self.id,
            (&self.hand.items)
                .into_iter()
                .map(|d| d.val.int())
                .collect::<Vec<u32>>()
        )
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

    // Gets the actual number of dice around the table, allowing for wildcards.
    fn num_logical_dice(&self, val: DieVal) -> usize {
        if val == DieVal::One {
            self.num_dice(DieVal::One)
        } else {
            self.num_dice(DieVal::One) + self.num_dice(val)
        }
    }

    fn num_dice_per_val(&self) -> HashMap<DieVal, usize> {
        c! { val.clone() => self.num_dice(val), for val in DieVal::all().into_iter() }
    }

    // Gets the best bet above a certain bet.
    // If no bet is better than Perudo then we return this.
    fn best_outcome_above(&self, bet: &Bet, total_num_dice: usize) -> TurnOutcome {
        // Create pairs of all possible outcomes sorted by probability.
        let mut outcomes = vec![
            (TurnOutcome::Perudo, bet.perudo_prob(total_num_dice, self)),
            (TurnOutcome::Palafico, bet.palafico_prob(total_num_dice, self)),
        ];
        outcomes.extend(bet.all_above(total_num_dice)
            .into_iter()
            .map(|b| (TurnOutcome::Bet(b.clone()), b.prob(total_num_dice, self)))
            .collect::<Vec<(TurnOutcome, f64)>>());

        // HACK - get an arbitrary one of the best outcomes.
        outcomes.sort_by(|a, b| ((a.1 * 1000000.0) as u64).cmp(&((b.1 * 1000000.0) as u64)));
        let best_p = outcomes[outcomes.len() - 1].1;
        let best_outcomes = outcomes.into_iter().filter(|a| a.1 == best_p).map(|a| a.0).collect::<Vec<TurnOutcome>>();
        let mut rng = thread_rng();
        best_outcomes.choose(&mut rng).unwrap().clone()
    }

    // Gets all bets ordered by probability.
    fn ordered_bets(&self, total_num_dice: usize) -> Vec<Bet> {
        let mut bets = Bet::all(total_num_dice)
            .into_iter()
            // TODO: Remove awful hack to get around lack of Ord on f64 and therefore no sort().
            .map(|b| ((100000.0 * b.prob(total_num_dice, self)) as u64, b))
            .collect::<Vec<(u64, Bet)>>();
        bets.sort_by(|a, b| a.0.cmp(&b.0));
        bets.into_iter().map(|x| x.1).collect::<Vec<Bet>>()
    }

    // Pick the best bet from those given.
    // TODO: Better than random choice from equally likely bets.
    fn best_first_bet(&self, total_num_dice: usize) -> Bet {
        let bets = self.first_bets(total_num_dice);
        let max_prob = bets[bets.len() - 1].prob(total_num_dice, self);
        let best_bets = bets.into_iter()
            .filter(|b| b.prob(total_num_dice, self) == max_prob)
            .collect::<Vec<Bet>>();
        let mut rng = thread_rng();
        best_bets.choose(&mut rng).unwrap().clone()
    }

    // Get the allowed first bets - everything but ones.
    // Bets are ordered by their probability of occuring.
    fn first_bets(&self, total_num_dice: usize) -> Vec<Bet> {
        self
            .ordered_bets(total_num_dice)
            .into_iter()
            .filter(|b| b.value != DieVal::One)
            .collect::<Vec<Bet>>()
    }

    fn play(&self, game: &Game, current_outcome: &TurnOutcome) -> TurnOutcome {
        if self.human {
            // TODO: More elegant way of implementing multiple play strategies.
            return self.human_play(game, current_outcome);
        }

        let total_num_dice = game.total_num_dice();
        match current_outcome {
            TurnOutcome::First => TurnOutcome::Bet(self.best_first_bet(total_num_dice)),
            TurnOutcome::Bet(current_bet) => self.best_outcome_above(current_bet, total_num_dice),
            TurnOutcome::Perudo => panic!(),
            TurnOutcome::Palafico => panic!(),
            TurnOutcome::Win => panic!(),
        }
    }

    // TODO: Make this a play function on some trait.
    fn human_play(&self, game: &Game, current_outcome: &TurnOutcome) -> TurnOutcome {
        loop {
            info!(
                "Dice left: {:?} ({})",
                game.num_dice_per_player(),
                game.total_num_dice()
            );
            info!("Hand for Player {}", self);
            match current_outcome {
                TurnOutcome::First => info!("Enter bet (2.6=two sixes):"),
                TurnOutcome::Bet(_) => info!("Enter bet (2.6=two sixes, p=perudo, pal=palafico):"),
                TurnOutcome::Perudo => panic!(),
                TurnOutcome::Palafico => panic!(),
                TurnOutcome::Win => panic!(),
            };

            let mut line = String::new();
            io::stdin()
                .read_line(&mut line)
                .expect("Failed to read input");
            let line = line.trim();

            if line == "p" {
                return TurnOutcome::Perudo;
            }
            if line == "pal" {
                return TurnOutcome::Palafico;
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
                None => continue,
            };

            let value = match split.next() {
                Some(v) => match v.parse::<usize>() {
                    Ok(v) => v,
                    Err(e) => {
                        info!("{}", e);
                        continue;
                    }
                },
                None => continue,
            };

            // Either return a valid bet or take input again.
            let bet = Bet {
                value: DieVal::from_usize(value),
                quantity: quantity,
            };
            return match current_outcome {
                TurnOutcome::First => TurnOutcome::Bet(bet),
                TurnOutcome::Bet(current_bet) => {
                    if bet > *current_bet {
                        return TurnOutcome::Bet(bet);
                    } else {
                        continue;
                    }
                }
                TurnOutcome::Perudo => panic!(),
                TurnOutcome::Palafico => panic!(),
                TurnOutcome::Win => panic!(),
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
    // Generate all possible bets.
    fn all(num_dice: usize) -> Vec<Self> {
        iproduct!(DieVal::all().into_iter(), 1..=num_dice)
            .map(|(value, quantity)| Bet {
                value: value,
                quantity: quantity,
            })
            .collect::<Vec<Bet>>()
    }

    fn all_without_ones(num_dice: usize) -> Vec<Self> {
        Bet::all(num_dice)
            .into_iter()
            .filter(|b| b.value != DieVal::One)
            .collect::<Vec<Bet>>()
    }

    // Get all possible bets above the one given.
    fn all_above(&self, num_dice: usize) -> Vec<Self> {
        // Generate all bets and filter down to only those which are greater than the one given.
        Bet::all(num_dice)
            .into_iter()
            .filter(|b| b > self)
            .collect::<Vec<Bet>>()
    }

    // Gets the probability that this bet is incorrect as far as the given player is concerned.
    fn perudo_prob(&self, total_num_dice: usize, player: &Player) -> f64 {
        1.0 - self.prob(total_num_dice, player) 
    }

    // Gets the probability that this bet is exactly correct as far as the given player is
    // concerned.
    fn palafico_prob(&self, total_num_dice: usize, player: &Player) -> f64 {
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
    fn prob(&self, total_num_dice: usize, player: &Player) -> f64 {
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

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum TurnOutcome {
    First,
    Bet(Bet),
    Perudo,
    Palafico,
    Win,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    players: Vec<Player>,
    current_index: usize,
    current_outcome: TurnOutcome,
    last_bet: Bet,
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Hands: {:?}",
            (&self.players)
                .into_iter()
                .map(|p| format!("{}", p))
                .collect::<Vec<String>>()
                .join(" | ")
        )
    }
}

impl Game {
    fn new(num_players: usize, human_indices: HashSet<usize>) -> Self {
        let mut game = Self {
            players: Vec::new(),
            current_index: 0,
            current_outcome: TurnOutcome::First,
            // TODO: Remove hack via an Option.
            last_bet: Bet {
                value: DieVal::One,
                quantity: 0,
            },
        };

        for id in 0..num_players {
            let human = human_indices.contains(&id);
            let player = Player::new(id, human);
            game.players.push(player);
        }

        game
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
        bet <= &max_correct_bet
    }

    fn is_exactly_correct(&self, bet: &Bet) -> bool {
        self.num_logical_dice(&bet.value) == bet.quantity
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
        loop {
            self.run_turn(None);
            match self.current_outcome {
                TurnOutcome::Win => return,
                _ => continue,
            }
        }
    }

    // Runs a turn and either finishes or sets up for the next turn.
    // TODO: Split up to decouple the game logic from the RL input.
    fn run_turn(&mut self, agent_override: Option<&TurnOutcome>) {
        let player = &self.players[self.current_index];

        // Either get the action from the RL agent or the player.
        // TODO: Decouple this.
        self.current_outcome = match agent_override {
            Some(outcome) => outcome.clone(),
            None => player.play(self, &self.current_outcome),
        };

        // TODO: Include historic bets in the context given to the player.
        debug!("{}", self);
        match &self.current_outcome {
            TurnOutcome::Bet(bet) => {
                info!("Player {} bets {}", player.id, bet);
                self.last_bet = bet.clone();
                self.current_index = (self.current_index + 1) % self.num_players();
            }
            TurnOutcome::Perudo => {
                info!("Player {} calls Perudo", player.id);
                let loser_index: usize;
                let actual_amount = self.num_logical_dice(&self.last_bet.value);
                if self.is_correct(&self.last_bet) {
                    info!(
                        "Player {} is incorrect, there were {} {:?}s",
                        player.id, actual_amount, self.last_bet.value
                    );
                    loser_index = self.current_index;
                } else {
                    info!(
                        "Player {} is correct, there were {} {:?}s",
                        player.id, actual_amount, self.last_bet.value
                    );
                    loser_index =
                        (self.current_index + self.num_players() - 1) % self.num_players();
                };
                self.end_turn(loser_index);
            },
            TurnOutcome::Palafico => {
                info!("Player {} calls Palafico", player.id);
                let actual_amount = self.num_logical_dice(&self.last_bet.value);
                if self.is_exactly_correct(&self.last_bet) {
                    info!(
                        "Player {} is correct, there were {} {:?}s",
                        player.id, actual_amount, self.last_bet.value
                    );
                    let winner_index =
                        (self.current_index + self.num_players() - 1) % self.num_players();
                    self.end_turn_palafico(winner_index);
                } else {
                    info!(
                        "Player {} is incorrect, there were {} {:?}s",
                        player.id, actual_amount, self.last_bet.value
                    );
                    self.end_turn(self.current_index);
                }
            },
            TurnOutcome::First => panic!(),
            TurnOutcome::Win => panic!(),
        };
    }

    fn end_turn_palafico(&mut self, winner_index: usize) {
        let winner = &self.players[winner_index];
        // Refresh all players, winner gains a die.
        self.players = self
            .players
            .clone()
            .into_iter()
            .enumerate()
            .map(|(i, p)| {
                if i == winner_index && p.hand.items.len() < 5 {
                    info!("Player {} gains a die, now has {}", winner.id, p.hand.items.len() + 1);
                    p.with_one()
                } else {
                    p.refresh()
                }
            })
            .collect();
        self.current_index = winner_index;
        self.current_outcome = TurnOutcome::First;
    }

    // Ends the turn and sets the next turn up.
    fn end_turn(&mut self, loser_index: usize) {
        let loser = &self.players[loser_index];
        if loser.hand.items.len() == 1 {
            info!("Player {} is disqualified", loser.id);
            self.players.remove(loser_index);

            if self.players.len() > 1 {
                self.current_index = (loser_index % self.num_players()) as usize;
                self.current_outcome = TurnOutcome::First;
            } else {
                info!("Player {} wins!", self.players[0].id);
                self.current_outcome = TurnOutcome::Win;
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
            info!(
                "Player {} loses a die, now has {}",
                self.players[loser_index].id,
                self.players[loser_index].hand.items.len()
            );
            // Reset and prepare for the next turn.
            self.current_index = loser_index;
            self.current_outcome = TurnOutcome::First;
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

    if args.len() >= 3 {
        human_indices.insert(args[2].parse::<usize>().unwrap());
    }

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

        fn approx(x: f64, y: f64) {
            if (x - y).abs() > 0.001 {
                panic!("{} != {}", x, y);
            }
        }

        it "computes probability for bets" {
            // Create a player with a few of each.
            let _game = Game::new(0, HashSet::new());
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

            // Bets on Ones, given one in the hand.
            approx(1.0, bet(DieVal::One, 0).prob(6, &player));
            approx(1.0, bet(DieVal::One, 1).prob(6, &player));
            approx(1.0 / 6.0, bet(DieVal::One, 2).prob(6, &player));

            // We have two 2s in the hand already.
            approx(1.0, bet(DieVal::Two, 0).prob(6, &player));
            approx(1.0, bet(DieVal::Two, 1).prob(6, &player));
            approx(1.0, bet(DieVal::Two, 2).prob(6, &player));
            approx(1.0 / 3.0, bet(DieVal::Two, 3).prob(6, &player));

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
            let opponent_bet = &Bet {
                quantity: 4,
                value: DieVal::Six,
            };
            let best_outcome_above = player.best_outcome_above(opponent_bet, total_num_dice);
            assert_eq!(best_outcome_above, TurnOutcome::Bet(Bet {
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
            let opponent_bet = &Bet {
                quantity: 1,
                value: DieVal::Six,
            };
            let best_outcome_above = player.best_outcome_above(opponent_bet, total_num_dice);
            assert_eq!(best_outcome_above, TurnOutcome::Palafico);           
        }
    }

    describe "a game" {
        it "runs to completion" {
            let mut game = Game::new(6, HashSet::new());
            game.run();
        }
    }
}
