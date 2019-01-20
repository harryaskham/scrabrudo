/// Player definitions and human/CPU behaviour.
use crate::bet::*;
use crate::game::*;
use crate::hand::*;
use crate::hand::*;
use crate::testing;

use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;
use std::cmp::Ord;
use std::collections::HashMap;
use speculate::speculate;
use std::fmt;
use std::io;

/// Common behaviour for players of any ruleset.
/// TODO: Remove Perudo references from the common core.
pub trait RenamePlayer: fmt::Debug + fmt::Display {
    /// Gets the player's ID.
    fn id(&self) -> usize;

    /// A copy of the player with an item missing.
    fn without_one(&self) -> Box<RenamePlayer>;

    /// A copy of the player with an extra item.
    fn with_one(&self) -> Box<RenamePlayer>;

    /// A fresh instance of player with a new hand.
    fn refresh(&self) -> Box<RenamePlayer>;

    /// TODO: Figure out how to remove this hack and still allow trait objectification.
    fn cloned(&self) -> Box<RenamePlayer>;

    /// Gets the best turn outcome above a certain bet.
    fn best_outcome_above(&self, bet: &PerudoBet, total_num_dice: usize) -> TurnOutcome;

    /// Control logic for having a human play the game.
    fn human_play(&self, game: &Game, current_outcome: &TurnOutcome) -> TurnOutcome;

    /// Whether or not the player is controlled by a human.
    fn is_human(&self) -> bool;

    /// The total number of items in the hand.
    fn num_items(&self) -> usize;

    /// The total number of dice with the given explicit value (no wildcards).
    fn num_items_with(&self, val: DieVal) -> usize;

    /// Gets the actual number of dice around the table, allowing for wildcards.
    /// TODO: Remove DieVal reference.
    fn num_logical_items(&self, val: DieVal) -> usize;

    /// Given the game state, return this player's chosen outcome.
    fn play(&self, game: &Game, current_outcome: &TurnOutcome) -> TurnOutcome;
}

#[derive(Debug, Clone)]
pub struct Player {
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

impl RenamePlayer for Player {
    fn id(&self) -> usize {
        self.id
    }

    fn without_one(&self) -> Box<RenamePlayer> {
        Box::new(Player {
            id: self.id,
            human: self.human,
            hand: Hand::<Die>::new(self.hand.items.len() as u32 - 1),
        })
    }

    fn with_one(&self) -> Box<RenamePlayer> {
        Box::new(Player {
            id: self.id,
            human: self.human,
            hand: Hand::<Die>::new(self.hand.items.len() as u32 + 1),
        })
    }

    fn refresh(&self) -> Box<RenamePlayer> {
        Box::new(Player {
            id: self.id,
            human: self.human,
            hand: Hand::<Die>::new(self.hand.items.len() as u32),
        })
    }

    fn cloned(&self) -> Box<RenamePlayer> {
        Box::new(Player {
            id: self.id,
            human: self.human,
            hand: self.hand.clone(),
        })
    }

    fn is_human(&self) -> bool {
        self.human
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

    fn best_outcome_above(&self, bet: &PerudoBet, total_num_dice: usize) -> TurnOutcome {
        let state = &GameState {
            num_items: total_num_dice,
        };

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

    fn play(&self, game: &Game, current_outcome: &TurnOutcome) -> TurnOutcome {
        if self.is_human() {
            // TODO: More elegant way of implementing multiple play strategies.
            return self.human_play(game, current_outcome);
        }

        // TODO: Can almost make this fully generic, need to tie together e.g. PerudoPlayer,
        // PerudoBet, PerudoGame somehow.
        let total_num_dice = game.total_num_dice();
        let state = &GameState {
            num_items: total_num_dice,
        };
        match current_outcome {
            TurnOutcome::First => TurnOutcome::Bet(*PerudoBet::best_first_bet(state, self)),
            TurnOutcome::Bet(current_bet) => self.best_outcome_above(current_bet, total_num_dice),
            _ => panic!(),
        }
    }

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
                TurnOutcome::Perudo => panic!(),
                TurnOutcome::Palafico => panic!(),
                TurnOutcome::Win => panic!(),
            };
        }
    }
}

impl Player {
    pub fn new(id: usize, human: bool) -> Player {
        Player {
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
            let player = &Player {
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
            let opponent_bet = &PerudoBet {
                quantity: 4,
                value: DieVal::Six,
            };
            let best_outcome_above = player.best_outcome_above(opponent_bet, total_num_dice);
            assert_eq!(best_outcome_above, TurnOutcome::Bet(PerudoBet {
                quantity: 5,
                value: DieVal::Six,
            }));
        }

        it "calls palafico with no other option" {
            let player = &Player {
                id: 0,
                human: false,
                hand: Hand::<Die> {
                    items: vec![
                        Die{ val: DieVal::Six },
                    ],
                },
            };
            let total_num_dice = 2;
            let opponent_bet = &PerudoBet {
                quantity: 1,
                value: DieVal::Six,
            };
            let best_outcome_above = player.best_outcome_above(opponent_bet, total_num_dice);
            assert_eq!(best_outcome_above, TurnOutcome::Palafico);
        }
    }
}
