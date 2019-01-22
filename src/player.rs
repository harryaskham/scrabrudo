/// Player definitions and human/CPU behaviour.
use crate::bet::*;
use crate::die::*;
use crate::game::*;
use crate::hand::*;
use crate::testing;

use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;
use speculate::speculate;
use std::cmp::Ord;
use std::collections::HashMap;
use std::fmt;
use std::io;

/// Common behaviour for players of any ruleset.
/// TODO: Remove Perudo references from the common core.
pub trait Player: fmt::Debug + fmt::Display {
    /// The type of thing this player holds.
    type V: Holdable;

    /// The type determining the bet to be used.
    type B: Bet;

    /// Gets the player's ID.
    fn id(&self) -> usize;

    /// Is the player human?
    fn human(&self) -> bool;

    /// A copy of the player with an item missing.
    fn without_one(&self) -> Box<Player<B = Self::B, V = Self::V>>;

    /// A copy of the player with an extra item.
    fn with_one(&self) -> Box<Player<B = Self::B, V = Self::V>>;

    /// A fresh instance of player with a new hand.
    fn refresh(&self) -> Box<Player<B = Self::B, V = Self::V>>;

    /// TODO: Figure out how to remove this hack and still allow trait objectification.
    fn cloned(&self) -> Box<Player<B = Self::B, V = Self::V>>;

    /// Gets the best turn outcome above a certain bet.
    fn best_outcome_above(&self, state: &GameState, bet: &Self::B) -> TurnOutcome<Self::B>;

    /// The player's hand.
    fn hand(&self) -> &Hand<Self::V>;

    /// The total number of items in the hand.
    fn num_items(&self) -> usize;

    /// The actual  items in the hand.
    fn items(&self) -> &Vec<Self::V>;

    /// The total number of dice with the given explicit value (no wildcards).
    fn num_items_with(&self, val: Self::V) -> usize;

    /// Gets the actual number of dice around the table, allowing for wildcards.
    fn num_logical_items(&self, val: Self::V) -> usize;

    /// Given the game state, return this player's chosen outcome.
    fn play(
        &self,
        state: &GameState,
        current_outcome: &TurnOutcome<Self::B>,
    ) -> TurnOutcome<Self::B>;

    /// Control logic for having a human play the game.
    fn human_play(
        &self,
        state: &GameState,
        current_outcome: &TurnOutcome<Self::B>,
    ) -> TurnOutcome<Self::B>;
}

#[derive(Debug, Clone)]
pub struct PerudoPlayer {
    pub id: usize,
    pub hand: Hand<Die>,
    pub human: bool,
}

impl PartialEq for PerudoPlayer {
    fn eq(&self, other: &PerudoPlayer) -> bool {
        // TODO: Better equality for PerudoPlayers.
        self.id == other.id
    }
}

impl Eq for PerudoPlayer {}

impl fmt::Display for PerudoPlayer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}: {:?}",
            self.id,
            (&self.hand.items)
                .into_iter()
                .map(|d| d.int())
                .collect::<Vec<u32>>()
        )
    }
}

impl Player for PerudoPlayer {
    type V = Die;
    type B = PerudoBet;

    fn id(&self) -> usize {
        self.id
    }

    fn human(&self) -> bool {
        self.human
    }

    /// TODO: These methods can all move to the base now, predicated on our V type.
    /// The blocker is we cannot have a new() func for a Trait Object.
    fn without_one(&self) -> Box<Player<B = PerudoBet, V = Die>> {
        Box::new(PerudoPlayer {
            id: self.id(),
            human: self.human(),
            hand: Hand::<Die>::new(self.num_items() as u32 - 1),
        })
    }

    fn with_one(&self) -> Box<Player<B = PerudoBet, V = Die>> {
        Box::new(PerudoPlayer {
            id: self.id(),
            human: self.human(),
            hand: Hand::<Die>::new(self.num_items() as u32 + 1),
        })
    }

    fn refresh(&self) -> Box<Player<B = PerudoBet, V = Die>> {
        Box::new(PerudoPlayer {
            id: self.id(),
            human: self.human(),
            hand: Hand::<Die>::new(self.num_items() as u32),
        })
    }

    fn cloned(&self) -> Box<Player<B = PerudoBet, V = Die>> {
        Box::new(PerudoPlayer {
            id: self.id(),
            human: self.human(),
            hand: self.hand().clone(),
        })
    }

    fn hand(&self) -> &Hand<Self::V> {
        &self.hand
    }

    fn num_items(&self) -> usize {
        self.hand.items.len()
    }

    fn items(&self) -> &Vec<Self::V> {
        &self.hand.items
    }

    fn num_items_with(&self, val: Die) -> usize {
        (&self.hand.items)
            .into_iter()
            .filter(|&d| d == &val)
            .count()
    }

    fn num_logical_items(&self, val: Die) -> usize {
        if val == Die::One {
            self.num_items_with(Die::One)
        } else {
            self.num_items_with(Die::One) + self.num_items_with(val)
        }
    }

    /// TODO: We should be able to lift this, need a way to have common functionality between
    /// struct implementers.
    /// Composition thing that takes the cloned self?
    fn best_outcome_above(&self, state: &GameState, bet: &PerudoBet) -> TurnOutcome<Self::B> {
        // Create pairs of all possible outcomes sorted by probability.
        let mut outcomes = vec![
            (
                TurnOutcome::Perudo,
                bet.prob(state, ProbVariant::Perudo, self.cloned()),
            ),
            (
                TurnOutcome::Palafico,
                bet.prob(state, ProbVariant::Palafico, self.cloned()),
            ),
        ];
        outcomes.extend(
            bet.all_above(state)
                .into_iter()
                .map(|b| {
                    (
                        TurnOutcome::Bet(*b.clone()),
                        b.prob(state, ProbVariant::Bet, self.cloned()),
                    )
                })
                .collect::<Vec<(TurnOutcome<Self::B>, f64)>>(),
        );
        get_best_outcome::<PerudoBet>(&outcomes)
    }

    fn play(
        &self,
        state: &GameState,
        current_outcome: &TurnOutcome<Self::B>,
    ) -> TurnOutcome<Self::B> {
        if self.human() {
            return self.human_play(state, current_outcome);
        }
        match current_outcome {
            TurnOutcome::First => {
                TurnOutcome::Bet(*PerudoBet::best_first_bet(state, self.cloned()))
            }
            TurnOutcome::Bet(current_bet) => self.best_outcome_above(state, current_bet),
            _ => panic!(),
        }
    }

    fn human_play(
        &self,
        state: &GameState,
        current_outcome: &TurnOutcome<Self::B>,
    ) -> TurnOutcome<Self::B> {
        loop {
            info!(
                "Dice left: {:?} ({})",
                state.num_items_per_player, state.total_num_items
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
            let bet = PerudoBet {
                value: Die::from_usize(value),
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
                _ => panic!(),
            };
        }
    }
}

impl PerudoPlayer {
    pub fn new(id: usize, human: bool) -> PerudoPlayer {
        PerudoPlayer {
            id: id,
            human: human,
            hand: Hand::<Die>::new(5),
        }
    }
}

/// Gets one of the arbitrarily best outcomes from a list of (outcome,p) pairs.
fn get_best_outcome<B: Bet>(outcomes: &Vec<(TurnOutcome<B>, f64)>) -> TurnOutcome<B> {
    let mut outcomes = outcomes.clone();
    outcomes.sort_by(|a, b| ((a.1 * 1000000.0) as u64).cmp(&((b.1 * 1000000.0) as u64)));
    let best_p = outcomes[outcomes.len() - 1].1;
    let best_outcomes = outcomes
        .into_iter()
        .filter(|a| a.1 == best_p)
        .map(|a| a.0)
        .collect::<Vec<TurnOutcome<B>>>();
    let mut rng = thread_rng();
    best_outcomes.choose(&mut rng).unwrap().clone()
}

speculate! {
    before {
        testing::set_up();
    }

    describe "perudo player" {
        it "generates the most likely bet" {
            let player = &PerudoPlayer {
                id: 0,
                human: false,
                hand: Hand::<Die> {
                    items: vec![
                        Die::Six,
                        Die::Six,
                        Die::Six,
                        Die::Six,
                        Die::Six
                    ],
                },
            };
            let state = &GameState {
                total_num_items: 5,
                num_items_per_player: vec![5],
            };
            let opponent_bet = &PerudoBet {
                quantity: 4,
                value: Die::Six,
            };
            let best_outcome_above = player.best_outcome_above(state, opponent_bet);
            assert_eq!(best_outcome_above, TurnOutcome::Bet(PerudoBet {
                quantity: 5,
                value: Die::Six,
            }));
        }

        it "calls palafico with no other option" {
            let player = &PerudoPlayer {
                id: 0,
                human: false,
                hand: Hand::<Die> {
                    items: vec![
                        Die::Six
                    ],
                },
            };
            let state = &GameState {
                total_num_items: 2,
                num_items_per_player: vec![1, 1],
            };
            let opponent_bet = &PerudoBet {
                quantity: 1,
                value: Die::Six,
            };
            let best_outcome_above = player.best_outcome_above(state, opponent_bet);
            assert_eq!(best_outcome_above, TurnOutcome::Palafico);
        }
    }
}
