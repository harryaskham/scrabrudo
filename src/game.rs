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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    pub players: Vec<Player>,
    pub current_index: usize,
    pub current_outcome: TurnOutcome,
    pub last_bet: PerudoBet,
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

// TODO: Remove this - required for ordering purposes but should have a minimum value via an enum.
// This can also just be an option instead.
fn hacky_first_bet() -> PerudoBet {
    return PerudoBet {
        value: DieVal::One,
        quantity: 0,
    };
}

impl Game {
    pub fn new(num_players: usize, human_indices: HashSet<usize>) -> Self {
        let mut game = Self {
            players: Vec::new(),
            current_index: 0,
            current_outcome: TurnOutcome::First,
            // TODO: Remove hack via an Option.
            last_bet: hacky_first_bet(),
        };

        for id in 0..num_players {
            let human = human_indices.contains(&id);
            let player = Player::new(id, human);
            game.players.push(player);
        }

        game
    }

    pub fn num_dice_per_val(&self) -> HashMap<DieVal, usize> {
        c! { val.clone() => self.num_dice(&val), for val in DieVal::all().into_iter() }
    }

    pub fn num_dice(&self, val: &DieVal) -> usize {
        (&self.players)
            .into_iter()
            .map(|p| &p.hand.items)
            .flatten()
            .filter(|d| &d.val() == val)
            .count()
    }

    // Gets the actual number of dice around the table, allowing for wildcards.
    pub fn num_logical_dice(&self, val: &DieVal) -> usize {
        if val == &DieVal::One {
            self.num_dice(&DieVal::One)
        } else {
            self.num_dice(&DieVal::One) + self.num_dice(val)
        }
    }

    // TODO: Candidate for moving into Bet
    pub fn is_correct(&self, bet: &PerudoBet) -> bool {
        let max_correct_bet = PerudoBet {
            value: bet.value.clone(),
            quantity: self.num_logical_dice(&bet.value),
        };
        bet <= &max_correct_bet
    }

    // TODO: Candidate for moving into Bet
    pub fn is_exactly_correct(&self, bet: &PerudoBet) -> bool {
        self.num_logical_dice(&bet.value) == bet.quantity
    }

    pub fn num_dice_per_player(&self) -> Vec<usize> {
        self.players
            .clone()
            .into_iter()
            .map(|p| p.hand.items.len())
            .collect()
    }

    pub fn total_num_dice(&self) -> usize {
        self.num_dice_per_player().iter().sum()
    }

    // Runs a turn and either finishes or sets up for the next turn.
    // TODO: Split up to decouple the game logic from the RL input.
    pub fn run_turn(&self) -> Game {
        let player = &self.players[self.current_index];

        // Get the current state based on this player's move.
        let current_outcome = player.play(self, &self.current_outcome);

        // TODO: Include historic bets in the context given to the player.
        debug!("{}", self);
        match current_outcome {
            TurnOutcome::Bet(bet) => {
                info!("Player {} bets {}", player.id, bet);
                let last_bet = bet.clone();
                return Game {
                    players: self.players.clone(),
                    current_index: (self.current_index + 1) % self.players.len(),
                    current_outcome: TurnOutcome::Bet(bet.clone()),
                    last_bet: last_bet,
                };
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
                        (self.current_index + self.players.len() - 1) % self.players.len();
                };
                return self.end_turn(loser_index);
            }
            TurnOutcome::Palafico => {
                info!("Player {} calls Palafico", player.id);
                let actual_amount = self.num_logical_dice(&self.last_bet.value);
                if self.is_exactly_correct(&self.last_bet) {
                    info!(
                        "Player {} is correct, there were {} {:?}s",
                        player.id, actual_amount, self.last_bet.value
                    );
                    return self.end_turn_palafico(self.current_index);
                } else {
                    info!(
                        "Player {} is incorrect, there were {} {:?}s",
                        player.id, actual_amount, self.last_bet.value
                    );
                    return self.end_turn(self.current_index);
                }
            }
            _ => panic!(),
        };
    }

    /// Ends the turn in Palafico and returns the new game state.
    pub fn end_turn_palafico(&self, winner_index: usize) -> Game {
        let winner = &self.players[winner_index];
        // Refresh all players, winner gains a die.
        let players = self
            .players
            .clone()
            .into_iter()
            .enumerate()
            .map(|(i, p)| {
                if i == winner_index && p.hand.items.len() < 5 {
                    info!(
                        "Player {} gains a die, now has {}",
                        winner.id,
                        p.hand.items.len() + 1
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
            last_bet: hacky_first_bet(),
        };
    }

    /// Gets a cloned refreshed view on the players.
    fn refreshed_players(&self) -> Vec<Player> {
        self.players
            .clone()
            .into_iter()
            .map(|p| p.refresh())
            .collect::<Vec<Player>>()
    }

    /// Gets the players refreshed with one player losing.
    fn refreshed_players_with_loss(&self, loser_index: usize) -> Vec<Player> {
        self.players
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
            .collect::<Vec<Player>>()
    }

    /// Ends the turn and returns the new game state.
    pub fn end_turn(&self, loser_index: usize) -> Game {
        let loser = &self.players[loser_index];
        if loser.hand.items.len() == 1 {
            info!("Player {} is disqualified", loser.id);

            // Clone the players with new hands, without the loser.
            let mut players = self.refreshed_players();
            players.remove(loser_index);

            if players.len() > 1 {
                return Game {
                    players: players.clone(),
                    current_index: (loser_index % players.len()) as usize,
                    current_outcome: TurnOutcome::First,
                    last_bet: hacky_first_bet(),
                };
            } else {
                info!("Player {} wins!", players[0].id);
                return Game {
                    players: players,
                    current_index: 0,
                    current_outcome: TurnOutcome::Win,
                    last_bet: hacky_first_bet(),
                };
            }
        } else {
            // Refresh all players, loser loses an item.
            let players = self.refreshed_players_with_loss(loser_index);
            info!(
                "Player {} loses a die, now has {}",
                players[loser_index].id,
                players[loser_index].hand.items.len()
            );

            // Reset and prepare for the next turn.
            return Game {
                players: players,
                current_index: loser_index,
                current_outcome: TurnOutcome::First,
                last_bet: hacky_first_bet(),
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
