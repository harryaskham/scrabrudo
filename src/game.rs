/// Game logic.
use crate::bet::*;
use crate::hand::*;
use crate::player::*;
use crate::testing;

use speculate::speculate;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;

// TODO: PerudoTurnOutcome and make a more general version when making Game variant-agnostic.
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum TurnOutcome {
    First,
    Bet(PerudoBet),
    Perudo,
    Palafico,
    Win,
}

/// An export of the state of the game required by Bets/Players to make progress.
pub struct GameState {
    /// The total number of items left around the table.
    pub num_items: usize,
}

pub struct Game {
    pub players: Vec<Box<dyn RenamePlayer>>,
    pub current_index: usize,
    pub current_outcome: TurnOutcome,
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
    pub fn new(num_players: usize, human_indices: HashSet<usize>) -> Self {
        let mut game = Self {
            players: Vec::new(),
            current_index: 0,
            current_outcome: TurnOutcome::First,
        };

        for id in 0..num_players {
            let human = human_indices.contains(&id);
            let player = Player::new(id, human);
            game.players.push(Box::new(player));
        }

        game
    }

    pub fn num_items_with(&self, val: DieVal) -> usize {
        self.players
            .iter()
            .map(|p| p.num_items_with(val.clone()))
            .sum()
    }

    // Gets the actual number of dice around the table, allowing for wildcards.
    pub fn num_logical_items(&self, val: DieVal) -> usize {
        if val == DieVal::One {
            self.num_items_with(DieVal::One)
        } else {
            self.num_items_with(DieVal::One) + self.num_items_with(val)
        }
    }

    // TODO: Candidate for moving into Bet
    pub fn is_correct(&self, bet: &PerudoBet) -> bool {
        let max_correct_bet = PerudoBet {
            value: bet.value.clone(),
            quantity: self.num_logical_items(bet.value.clone()),
        };
        bet <= &max_correct_bet
    }

    // TODO: Candidate for moving into Bet
    pub fn is_exactly_correct(&self, bet: &PerudoBet) -> bool {
        self.num_logical_items(bet.value.clone()) == bet.quantity
    }

    pub fn num_dice_per_player(&self) -> Vec<usize> {
        self.players
            .iter()
            .map(|p| p.num_items())
            .collect()
    }

    pub fn total_num_dice(&self) -> usize {
        self.num_dice_per_player().iter().sum()
    }

    // Gets the last bet issued.
    pub fn last_bet(&self) -> PerudoBet {
        match &self.current_outcome {
            // TODO: Remove this hacky thing with e.g. PerudoBet::min()
            TurnOutcome::First => *PerudoBet::smallest(),
            TurnOutcome::Bet(bet) => bet.clone(),
            _ => panic!(),
        }
    }

    // Runs a turn and either finishes or sets up for the next turn.
    pub fn run_turn(&self) -> Game {
        let last_bet = self.last_bet();

        // Get the current state based on this player's move.
        let player = &self.players[self.current_index];
        let current_outcome = player.play(self, &self.current_outcome);

        // TODO: Include historic bets in the context given to the player.
        debug!("{}", self);
        match current_outcome {
            TurnOutcome::Bet(bet) => {
                info!("Player {} bets {}", player.id(), bet);
                return Game {
                    players: self.cloned_players(),
                    current_index: (self.current_index + 1) % self.players.len(),
                    current_outcome: TurnOutcome::Bet(bet.clone()),
                };
            }
            TurnOutcome::Perudo => {
                info!("Player {} calls Perudo", player.id());
                let loser_index: usize;
                let actual_amount = self.num_logical_items(last_bet.value.clone());
                if self.is_correct(&last_bet) {
                    info!(
                        "Player {} is incorrect, there were {} {:?}s",
                        player.id(), actual_amount, last_bet.value
                    );
                    loser_index = self.current_index;
                } else {
                    info!(
                        "Player {} is correct, there were {} {:?}s",
                        player.id(), actual_amount, last_bet.value
                    );
                    loser_index =
                        (self.current_index + self.players.len() - 1) % self.players.len();
                };
                self.with_end_turn(loser_index)
            }
            TurnOutcome::Palafico => {
                info!("Player {} calls Palafico", player.id());
                let actual_amount = self.num_logical_items(last_bet.value.clone());
                if self.is_exactly_correct(&last_bet) {
                    info!(
                        "Player {} is correct, there were {} {:?}s",
                        player.id(), actual_amount, last_bet.value
                    );
                    return self.with_end_turn_palafico(self.current_index);
                } else {
                    info!(
                        "Player {} is incorrect, there were {} {:?}s",
                        player.id(), actual_amount, last_bet.value
                    );
                    self.with_end_turn(self.current_index)
                }
            }
            _ => panic!(),
        }
    }

    /// Ends the turn in Palafico and returns the new game state.
    pub fn with_end_turn_palafico(&self, winner_index: usize) -> Game {
        let winner = &self.players[winner_index];
        // Refresh all players, winner gains a die.
        let players = self
            .players
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if i == winner_index && p.num_items() < 5 {
                    info!(
                        "Player {} gains a die, now has {}",
                        winner.id(),
                        p.num_items() + 1
                    );
                    p.with_one()
                } else {
                    p.refresh()
                }
            })
            .collect();
        return Game {
            players: players,
            current_index: winner_index,
            current_outcome: TurnOutcome::First,
        };
    }

    /// Gets a cloned refreshed view on the players.
    fn refreshed_players(&self) -> Vec<Box<dyn RenamePlayer>> {
        self.players
            .iter()
            .map(|p| p.refresh())
            .collect()
    }

    /// Clones players without touching their hands.
    fn cloned_players(&self) -> Vec<Box<dyn RenamePlayer>> {
        self.players
            .iter()
            .map(|p| p.cloned())
            .collect()
    }

    /// Gets the players refreshed with one player losing.
    fn refreshed_players_with_loss(&self, loser_index: usize) -> Vec<Box<dyn RenamePlayer>> {
        self.players
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if i == loser_index {
                    p.without_one()
                } else {
                    p.refresh()
                }
            })
            .collect()
    }

    /// Ends the turn and returns the new game state.
    pub fn with_end_turn(&self, loser_index: usize) -> Game {
        let loser = &self.players[loser_index];
        if loser.num_items() == 1 {
            info!("Player {} is disqualified", loser.id());

            // Clone the players with new hands, without the loser.
            let mut players = self.refreshed_players();
            players.remove(loser_index);
            let current_index = (loser_index % players.len()) as usize;

            if players.len() > 1 {
                return Game {
                    players: players,
                    current_index: current_index,
                    current_outcome: TurnOutcome::First,
                };
            } else {
                info!("Player {} wins!", players[0].id());
                return Game {
                    players: players,
                    current_index: 0,
                    current_outcome: TurnOutcome::Win,
                };
            }
        } else {
            // Refresh all players, loser loses an item.
            let players = self.refreshed_players_with_loss(loser_index);
            info!(
                "Player {} loses a die, now has {}",
                players[loser_index].id(),
                players[loser_index].num_items()
            );

            // Reset and prepare for the next turn.
            return Game {
                players: players,
                current_index: loser_index,
                current_outcome: TurnOutcome::First,
            };
        }
    }
}

speculate! {
    before {
        testing::set_up();
    }

    describe "a game" {
        it "runs to completion" {
            let mut game = Game::new(6, HashSet::new());
            loop {
                game = game.run_turn();
                match game.current_outcome {
                    TurnOutcome::Win => return,
                    _ => continue,
                }
            }
        }
    }
}
