/// Player definitions and human/CPU behaviour.
use crate::bet::*;
use crate::game::*;
use crate::hand::*;
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
    /// The type determining the bet to be used.
    type B: Bet;

    /// Gets the player's ID.
    fn id(&self) -> usize;

    /// A copy of the player with an item missing.
    fn without_one(&self) -> Box<Player<B=Self::B>>;

    /// A copy of the player with an extra item.
    fn with_one(&self) -> Box<Player<B=Self::B>>;

    /// A fresh instance of player with a new hand.
    fn refresh(&self) -> Box<Player<B=Self::B>>;

    /// TODO: Figure out how to remove this hack and still allow trait objectification.
    fn cloned(&self) -> Box<Player<B=Self::B>>;

    /// Gets the best turn outcome above a certain bet.
    fn best_outcome_above(&self, state: &GameState, bet: &Self::B) -> TurnOutcome;

    /// The total number of items in the hand.
    fn num_items(&self) -> usize;

    /// The total number of dice with the given explicit value (no wildcards).
    fn num_items_with(&self, val: DieVal) -> usize;

    /// Gets the actual number of dice around the table, allowing for wildcards.
    /// TODO: Remove DieVal reference.
    fn num_logical_items(&self, val: DieVal) -> usize;

    /// Given the game state, return this player's chosen outcome.
    fn play(&self, state: &GameState, current_outcome: &TurnOutcome) -> TurnOutcome;

    /// Control logic for having a human play the game.
    fn human_play(&self, state: &GameState, current_outcome: &TurnOutcome) -> TurnOutcome;
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
                .map(|d| d.val.int())
                .collect::<Vec<u32>>()
        )
    }
}

impl Player for PerudoPlayer {
    type B = PerudoBet;

    fn id(&self) -> usize {
        self.id
    }

    fn without_one(&self) -> Box<Player<B=PerudoBet>> {
        Box::new(PerudoPlayer {
            id: self.id,
            human: self.human,
            hand: Hand::<Die>::new(self.hand.items.len() as u32 - 1),
        })
    }

    fn with_one(&self) -> Box<Player<B=PerudoBet>> {
        Box::new(PerudoPlayer {
            id: self.id,
            human: self.human,
            hand: Hand::<Die>::new(self.hand.items.len() as u32 + 1),
        })
    }

    fn refresh(&self) -> Box<Player<B=PerudoBet>> {
        Box::new(PerudoPlayer {
            id: self.id,
            human: self.human,
            hand: Hand::<Die>::new(self.hand.items.len() as u32),
        })
    }

    fn cloned(&self) -> Box<Player<B=PerudoBet>> {
        Box::new(PerudoPlayer {
            id: self.id,
            human: self.human,
            hand: self.hand.clone(),
        })
    }

    fn num_items(&self) -> usize {
        self.hand.items.len()
    }

    fn num_items_with(&self, val: DieVal) -> usize {
        (&self.hand.items)
            .into_iter()
            .filter(|d| d.val() == val)
            .count()
    }

    fn num_logical_items(&self, val: DieVal) -> usize {
        if val == DieVal::One {
            self.num_items_with(DieVal::One)
        } else {
            self.num_items_with(DieVal::One) + self.num_items_with(val)
        }
    }

    fn best_outcome_above(&self, state: &GameState, bet: &PerudoBet) -> TurnOutcome {
        // Create pairs of all possible outcomes sorted by probability.
        let mut outcomes = vec![
            (
                TurnOutcome::Perudo,
                bet.prob(state, ProbVariant::Perudo, self),
            ),
            (
                TurnOutcome::Palafico,
                bet.prob(state, ProbVariant::Palafico, self),
            ),
        ];
        outcomes.extend(
            bet.all_above(state)
                .into_iter()
                .map(|b| {
                    (
                        TurnOutcome::Bet(*b.clone()),
                        b.prob(state, ProbVariant::Bet, self),
                    )
                })
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

    // TODO: Probably should move to game.rs
    fn play(&self, state: &GameState, current_outcome: &TurnOutcome) -> TurnOutcome {
        if self.human {
            // TODO: More elegant way of implementing multiple play strategies.
            return self.human_play(state, current_outcome);
        }

        // TODO: Can almost make this fully generic, need to tie together e.g. PerudoPlayer,
        // PerudoBet, PerudoGame somehow.
        match current_outcome {
            TurnOutcome::First => TurnOutcome::Bet(*PerudoBet::best_first_bet(state, self)),
            TurnOutcome::Bet(current_bet) => self.best_outcome_above(state, current_bet),
            _ => panic!(),
        }
    }

    // TODO: Probably should move to game.rs
    fn human_play(&self, state: &GameState, current_outcome: &TurnOutcome) -> TurnOutcome {
        loop {
            info!(
                "Dice left: {:?} ({})",
                state.num_items_per_player,
                state.total_num_items
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
                        Die{ val: DieVal::Six },
                        Die{ val: DieVal::Six },
                        Die{ val: DieVal::Six },
                        Die{ val: DieVal::Six },
                        Die{ val: DieVal::Six }
                    ],
                },
            };
            let state = &GameState {
                total_num_items: 5,
                num_items_per_player: vec![5],
            };
            let opponent_bet = &PerudoBet {
                quantity: 4,
                value: DieVal::Six,
            };
            let best_outcome_above = player.best_outcome_above(state, opponent_bet);
            assert_eq!(best_outcome_above, TurnOutcome::Bet(PerudoBet {
                quantity: 5,
                value: DieVal::Six,
            }));
        }

        it "calls palafico with no other option" {
            let player = &PerudoPlayer {
                id: 0,
                human: false,
                hand: Hand::<Die> {
                    items: vec![
                        Die{ val: DieVal::Six },
                    ],
                },
            };
            let state = &GameState {
                total_num_items: 2,
                num_items_per_player: vec![1, 1],
            };
            let opponent_bet = &PerudoBet {
                quantity: 1,
                value: DieVal::Six,
            };
            let best_outcome_above = player.best_outcome_above(state, opponent_bet);
            assert_eq!(best_outcome_above, TurnOutcome::Palafico);
        }
    }
}
