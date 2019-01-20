/// Player definitions and human/CPU behaviour.
use crate::bet::*;
use crate::game::*;
use crate::hand::*;
use crate::hand::*;

use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;
use std::cmp::Ord;
use std::collections::HashMap;
use std::fmt;
use std::io;

#[derive(Debug, Clone)]
pub struct Player {
    // TODO: Make private again and enforce immutability via with/without functions.
    pub id: usize,
    pub hand: Hand<Die>,
    pub human: bool,
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
    pub fn new(id: usize, human: bool) -> Self {
        Self {
            id: id,
            human: human,
            hand: Hand::<Die>::new(5),
        }
    }

    pub fn without_one(&self) -> Self {
        Self {
            id: self.id,
            human: self.human,
            hand: Hand::<Die>::new(self.hand.items.len() as u32 - 1),
        }
    }

    pub fn with_one(&self) -> Self {
        Self {
            id: self.id,
            human: self.human,
            hand: Hand::<Die>::new(self.hand.items.len() as u32 + 1),
        }
    }

    pub fn refresh(&self) -> Self {
        Self {
            id: self.id,
            human: self.human,
            hand: Hand::<Die>::new(self.hand.items.len() as u32),
        }
    }

    pub fn num_dice(&self, val: DieVal) -> usize {
        (&self.hand.items)
            .into_iter()
            .filter(|d| d.val() == val)
            .count()
    }

    // Gets the actual number of dice around the table, allowing for wildcards.
    pub fn num_logical_dice(&self, val: DieVal) -> usize {
        if val == DieVal::One {
            self.num_dice(DieVal::One)
        } else {
            self.num_dice(DieVal::One) + self.num_dice(val)
        }
    }

    pub fn num_dice_per_val(&self) -> HashMap<DieVal, usize> {
        c! { val.clone() => self.num_dice(val), for val in DieVal::all().into_iter() }
    }

    // Gets the best bet above a certain bet.
    // If no bet is better than Perudo then we return this.
    pub fn best_outcome_above(&self, bet: &Bet, total_num_dice: usize) -> TurnOutcome {
        // Create pairs of all possible outcomes sorted by probability.
        let mut outcomes = vec![
            (TurnOutcome::Perudo, bet.perudo_prob(total_num_dice, self)),
            (
                TurnOutcome::Palafico,
                bet.palafico_prob(total_num_dice, self),
            ),
        ];
        outcomes.extend(
            bet.all_above(total_num_dice)
                .into_iter()
                .map(|b| (TurnOutcome::Bet(b.clone()), b.prob(total_num_dice, self)))
                .collect::<Vec<(TurnOutcome, f64)>>(),
        );

        // HACK - get an arbitrary one of the best outcomes.
        outcomes.sort_by(|a, b| ((a.1 * 1000000.0) as u64).cmp(&((b.1 * 1000000.0) as u64)));
        let best_p = outcomes[outcomes.len() - 1].1;
        let best_outcomes = outcomes
            .into_iter()
            .filter(|a| a.1 == best_p)
            .map(|a| a.0)
            .collect::<Vec<TurnOutcome>>();
        let mut rng = thread_rng();
        best_outcomes.choose(&mut rng).unwrap().clone()
    }

    // Gets all bets ordered by probability.
    pub fn ordered_bets(&self, total_num_dice: usize) -> Vec<Bet> {
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
    pub fn best_first_bet(&self, total_num_dice: usize) -> Bet {
        let bets = self.first_bets(total_num_dice);
        let max_prob = bets[bets.len() - 1].prob(total_num_dice, self);
        let best_bets = bets
            .into_iter()
            .filter(|b| b.prob(total_num_dice, self) == max_prob)
            .collect::<Vec<Bet>>();
        let mut rng = thread_rng();
        best_bets.choose(&mut rng).unwrap().clone()
    }

    // Get the allowed first bets - everything but ones.
    // Bets are ordered by their probability of occuring.
    pub fn first_bets(&self, total_num_dice: usize) -> Vec<Bet> {
        self.ordered_bets(total_num_dice)
            .into_iter()
            .filter(|b| b.value != DieVal::One)
            .collect::<Vec<Bet>>()
    }

    pub fn play(&self, game: &Game, current_outcome: &TurnOutcome) -> TurnOutcome {
        if self.human {
            // TODO: More elegant way of implementing multiple play strategies.
            return self.human_play(game, current_outcome);
        }

        let total_num_dice = game.total_num_dice();
        match current_outcome {
            TurnOutcome::First => TurnOutcome::Bet(self.best_first_bet(total_num_dice)),
            TurnOutcome::Bet(current_bet) => self.best_outcome_above(current_bet, total_num_dice),
            _ => panic!(),
        }
    }

    // TODO: Make this a play function on some trait.
    pub fn human_play(&self, game: &Game, current_outcome: &TurnOutcome) -> TurnOutcome {
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
                _ => panic!(),
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
