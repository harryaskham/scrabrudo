/// Game logic.

use crate::player::*;
use crate::bet::*;
use crate::hand::*;

use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;

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
    pub fn new(num_players: usize, human_indices: HashSet<usize>) -> Self {
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

    pub fn num_players(&self) -> usize {
        self.players.len()
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

    pub fn is_correct(&self, bet: &Bet) -> bool {
        let max_correct_bet = Bet {
            value: bet.value.clone(),
            quantity: self.num_logical_dice(&bet.value),
        };
        bet <= &max_correct_bet
    }

    pub fn is_exactly_correct(&self, bet: &Bet) -> bool {
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

    pub fn run(&mut self) {
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
    pub fn run_turn(&mut self, agent_override: Option<&TurnOutcome>) {
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
                    self.end_turn_palafico(self.current_index);
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

    pub fn end_turn_palafico(&mut self, winner_index: usize) {
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
    pub fn end_turn(&mut self, loser_index: usize) {
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

