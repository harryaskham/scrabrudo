/// Game logic.
use crate::bet::*;
use crate::dict::*;
use crate::die::*;
use crate::hand::*;
use crate::player::*;
use crate::testing;
use crate::tile::*;

use speculate::speculate;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;

// TODO: PerudoTurnOutcome and make a more general version when making Game variant-agnostic.
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum TurnOutcome<B: Bet> {
    First,
    Bet(B),
    Perudo,
    Palafico,
    Win,
}

/// An export of the state of the game required by Bets/Players to make progress.
pub struct GameState {
    /// The total number of items left around the table.
    pub total_num_items: usize,

    /// The number of items remaining with each player.
    pub num_items_per_player: Vec<usize>,
}

/// Trait implemented by all game types.
/// Most rule-logic lives in the trait as it does not differ from game to game.
pub trait Game: Sized + fmt::Display {
    /// The associated value-type of a given hand item.
    type V: Holdable + Clone;

    /// The Bet type to use.
    type B: Bet<V = Self::V> + Clone;

    /// The associated type of a Player
    type P: Player<B = Self::B, V = Self::V>;

    /// Creates a new instance of the game.
    fn new(num_players: usize, items_per_player: usize, human_indices: HashSet<usize>) -> Self {
        let mut players = Vec::new();
        for id in 0..num_players {
            players.push(Self::create_player(
                id,
                items_per_player,
                human_indices.contains(&id),
            ));
        }
        Self::new_with(players, 0, TurnOutcome::First)
    }

    /// Creates a new instance with the given fields.
    fn new_with(
        players: Vec<Box<dyn Player<B = Self::B, V = Self::V>>>,
        current_index: usize,
        current_outcome: TurnOutcome<Self::B>,
    ) -> Self;

    /// Creates a new player.
    fn create_player(
        id: usize,
        items_per_player: usize,
        human: bool,
    ) -> Box<dyn Player<B = Self::B, V = Self::V>>;

    /// Gets a list of all the players.
    fn players(&self) -> &Vec<Box<dyn Player<B = Self::B, V = Self::V>>>;

    /// Gets the outcome of the turn currently being represented.
    fn current_outcome(&self) -> &TurnOutcome<Self::B>;

    /// Gets the index of the current player.
    fn current_index(&self) -> usize;

    /// Gets the actual number of dice around the table, allowing for wildcards.
    fn num_logical_items(&self, val: Self::V) -> usize;

    /// Whether or not the given bet is correct at the current state.
    fn is_correct(&self, bet: &Self::B) -> bool;

    /// Whether or not the given bet is precisely (Palafico-satisfyingly) correct at the current state.
    fn is_exactly_correct(&self, bet: &Self::B) -> bool;

    /// Gets a state representation of the game.
    fn state(&self) -> GameState {
        GameState {
            total_num_items: self.total_num_items(),
            num_items_per_player: self.num_items_per_player(),
        }
    }

    fn num_items_with(&self, val: Self::V) -> usize {
        self.players()
            .iter()
            .map(|p| p.num_items_with(val.clone()))
            .sum()
    }

    /// Gets the number of items remaining per player by index.
    fn num_items_per_player(&self) -> Vec<usize> {
        self.players().iter().map(|p| p.num_items()).collect()
    }

    /// Gets the total number of remaining items.
    fn total_num_items(&self) -> usize {
        self.num_items_per_player().iter().sum()
    }

    /// Gets a cloned refreshed view on the players.
    fn refreshed_players(&self) -> Vec<Box<dyn Player<B = Self::B, V = Self::V>>> {
        self.players().iter().map(|p| p.refresh()).collect()
    }

    /// Clones players without touching their hands.
    fn cloned_players(&self) -> Vec<Box<dyn Player<B = Self::B, V = Self::V>>> {
        self.players().iter().map(|p| p.cloned()).collect()
    }

    /// Gets the players refreshed with one player losing.
    fn refreshed_players_with_loss(
        &self,
        loser_index: usize,
    ) -> Vec<Box<dyn Player<B = Self::B, V = Self::V>>> {
        self.players()
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

    /// Gets the players refreshed with one player winning.
    fn refreshed_players_with_gain(
        &self,
        winner_index: usize,
    ) -> Vec<Box<dyn Player<B = Self::B, V = Self::V>>> {
        self.players()
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if i == winner_index && p.num_items() < 5 {
                    p.with_one()
                } else {
                    p.refresh()
                }
            })
            .collect()
    }

    /// Gets the last bet issued.
    fn last_bet(&self) -> Self::B {
        match self.current_outcome() {
            TurnOutcome::First => *Self::B::smallest(),
            TurnOutcome::Bet(bet) => bet.clone(),
            _ => panic!(),
        }
    }

    /// Ends the turn and returns the new game state.
    fn with_end_turn(&self, loser_index: usize) -> Self {
        let loser = &self.players()[loser_index];
        if loser.num_items() == 1 {
            info!("Player {} is disqualified", loser.id());

            // Clone the players with new hands, without the loser.
            let mut players = self.refreshed_players();
            players.remove(loser_index);
            let current_index = (loser_index % players.len()) as usize;

            if players.len() > 1 {
                return Self::new_with(players, current_index, TurnOutcome::First);
            } else {
                info!("Player {} wins!", players[0].id());
                return Self::new_with(players, 0, TurnOutcome::Win);
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
            return Self::new_with(players, loser_index, TurnOutcome::First);
        }
    }

    /// Ends the turn in Palafico and returns the new game state.
    fn with_end_turn_palafico(&self, winner_index: usize) -> Self {
        // Refresh all players, winner maybe gains a item.
        let players = self.refreshed_players_with_gain(winner_index);
        let winner = &players[winner_index];
        info!(
            "Player {} wins Palafico, now has {}",
            winner.id(),
            winner.num_items()
        );
        Self::new_with(players, winner_index, TurnOutcome::First)
    }

    /// Runs a turn and either finishes or sets up for the next turn, returning a full copy of
    /// the game in the new state.
    fn run_turn(&self) -> Self {
        let last_bet = self.last_bet();

        // Get the current state based on this player's move.
        let player = &self.players()[self.current_index()];
        let current_outcome = player.play(&self.state(), &self.current_outcome());

        // TODO: Include historic bets in the context given to the player.
        debug!("{}", self);
        match current_outcome {
            TurnOutcome::Bet(bet) => {
                info!("Player {} bets {}", player.id(), bet);
                Self::new_with(
                    self.cloned_players(),
                    (self.current_index() + 1) % self.players().len(),
                    TurnOutcome::Bet(bet.clone()),
                )
            }
            TurnOutcome::Perudo => {
                info!("Player {} calls Perudo", player.id());
                let loser_index: usize;
                if self.is_correct(&last_bet) {
                    loser_index = self.current_index();
                } else {
                    loser_index =
                        (self.current_index() + self.players().len() - 1) % self.players().len();
                };
                self.with_end_turn(loser_index)
            }
            TurnOutcome::Palafico => {
                info!("Player {} calls Palafico", player.id());
                if self.is_exactly_correct(&last_bet) {
                    self.with_end_turn_palafico(self.current_index())
                } else {
                    self.with_end_turn(self.current_index())
                }
            }
            _ => panic!(),
        }
    }
}

pub struct PerudoGame {
    pub players: Vec<Box<dyn Player<B = PerudoBet, V = Die>>>,
    pub current_index: usize,
    pub current_outcome: TurnOutcome<PerudoBet>,
}

impl fmt::Display for PerudoGame {
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

impl Game for PerudoGame {
    type V = Die;
    type B = PerudoBet;
    type P = PerudoPlayer;

    fn create_player(
        id: usize,
        items_per_player: usize,
        human: bool,
    ) -> Box<dyn Player<B = Self::B, V = Self::V>> {
        Box::new(PerudoPlayer {
            id: id,
            human: human,
            hand: Hand::<Die>::new(items_per_player as u32),
        })
    }

    fn players(&self) -> &Vec<Box<dyn Player<B = Self::B, V = Self::V>>> {
        &self.players
    }

    fn current_outcome(&self) -> &TurnOutcome<Self::B> {
        &self.current_outcome
    }

    fn current_index(&self) -> usize {
        self.current_index
    }

    fn new_with(
        players: Vec<Box<dyn Player<B = Self::B, V = Self::V>>>,
        current_index: usize,
        current_outcome: TurnOutcome<Self::B>,
    ) -> Self {
        Self {
            players: players,
            current_index: current_index,
            current_outcome: current_outcome,
        }
    }

    fn num_logical_items(&self, val: Die) -> usize {
        if val == Die::One {
            self.num_items_with(Die::One)
        } else {
            self.num_items_with(Die::One) + self.num_items_with(val)
        }
    }

    fn is_correct(&self, bet: &PerudoBet) -> bool {
        let max_correct_bet = PerudoBet {
            value: bet.value.clone(),
            quantity: self.num_logical_items(bet.value.clone()),
        };
        let is_correct = bet <= &max_correct_bet;

        // Log out the outcome.
        let actual_amount = self.num_logical_items(bet.value.clone());
        info!(
            "Bet was {}, there were {} {:?}s",
            if is_correct { "correct" } else { "incorrect" },
            actual_amount,
            bet.value
        );

        is_correct
    }

    fn is_exactly_correct(&self, bet: &PerudoBet) -> bool {
        let is_exactly_correct = self.num_logical_items(bet.value.clone()) == bet.quantity;

        // Log out the outcome.
        let actual_amount = self.num_logical_items(bet.value.clone());
        info!(
            "Bet was {}, there were {} {:?}s",
            if is_exactly_correct {
                "exactly correct"
            } else {
                "incorrect"
            },
            actual_amount,
            bet.value
        );

        is_exactly_correct
    }
}

pub struct ScrabrudoGame {
    pub players: Vec<Box<dyn Player<B = ScrabrudoBet, V = Tile>>>,
    pub current_index: usize,
    pub current_outcome: TurnOutcome<ScrabrudoBet>,
}

impl fmt::Display for ScrabrudoGame {
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

impl Game for ScrabrudoGame {
    type V = Tile;
    type B = ScrabrudoBet;
    type P = ScrabrudoPlayer;

    fn create_player(
        id: usize,
        items_per_player: usize,
        human: bool,
    ) -> Box<dyn Player<B = Self::B, V = Self::V>> {
        Box::new(ScrabrudoPlayer {
            id: id,
            human: human,
            hand: Hand::<Tile>::new(items_per_player as u32),
        })
    }

    fn players(&self) -> &Vec<Box<dyn Player<B = Self::B, V = Self::V>>> {
        &self.players
    }

    fn current_outcome(&self) -> &TurnOutcome<Self::B> {
        &self.current_outcome
    }

    fn current_index(&self) -> usize {
        self.current_index
    }

    fn new_with(
        players: Vec<Box<dyn Player<B = Self::B, V = Self::V>>>,
        current_index: usize,
        current_outcome: TurnOutcome<Self::B>,
    ) -> Self {
        Self {
            players: players,
            current_index: current_index,
            current_outcome: current_outcome,
        }
    }

    fn num_logical_items(&self, val: Tile) -> usize {
        // TODO: Update if we introduce blanks / wildcards.
        self.num_items_with(val)
    }

    fn is_correct(&self, bet: &ScrabrudoBet) -> bool {
        // A bet is correct if it's in the dictionary and it can be made from the tiles around the
        // table.
        if !ScrabbleDict::has_word(bet.as_word()) {
            info!("Spurious - we reject the bet because its not in-dict - should be checkignn all aangrams");
            return false;
        }

        let all_tiles = (&self.players)
            .iter()
            .map(|p| p.items())
            .flatten()
            .map(|t| t.clone())
            .collect::<Vec<Tile>>();
        let tile_counts = count_map(&bet.tiles);
        let all_tile_counts = count_map(&all_tiles);
        let mut is_correct = true;
        for (tile, count) in &tile_counts {
            let actual_count = match all_tile_counts.get(tile) {
                Some(c) => *c,
                None => 0,
            };
            if actual_count < *count {
                is_correct = false;
                break;
            }
        }

        // Log out the outcome.
        info!(
            "Bet was {}, {} is{}in {:?}",
            if is_correct { "correct" } else { "incorrect" },
            bet.as_word(),
            if is_correct { " " } else { " not " },
            all_tiles
        );

        is_correct
    }

    // TODO: Remove duplication with a helper here.
    fn is_exactly_correct(&self, bet: &ScrabrudoBet) -> bool {
        let all_tiles = (&self.players)
            .iter()
            .map(|p| p.items())
            .flatten()
            .map(|t| t.clone())
            .collect::<Vec<Tile>>();
        let tile_counts = count_map(&bet.tiles);
        let all_tile_counts = count_map(&all_tiles);
        let mut is_correct = true;
        for (tile, count) in &tile_counts {
            let actual_count = match all_tile_counts.get(tile) {
                Some(c) => *c,
                None => 0,
            };
            if actual_count != *count {
                is_correct = false;
                break;
            }
        }

        // Log out the outcome.
        info!(
            "Bet was {}, {} is{}exactly in {:?}",
            if is_correct { "correct" } else { "incorrect" },
            bet.as_word(),
            if is_correct { " " } else { " not " },
            all_tiles
        );

        is_correct
    }
}

speculate! {
    before {
        testing::set_up();
    }

    describe "a perudo game" {
        it "runs to completion" {
            let mut game = PerudoGame::new(6, 5, HashSet::new());
            loop {
                game = game.run_turn();
                match game.current_outcome {
                    TurnOutcome::Win => return,
                    _ => continue,
                }
            }
        }
    }

    describe "a scrabrudo game" {
        it "runs to completion" {
            let mut game = ScrabrudoGame::new(2, 2, HashSet::new());
            loop {
                game = game.run_turn();
                match game.current_outcome {
                    TurnOutcome::Win => return,
                    _ => continue,
                }
            }
        }

        it "constrains bet correctness including palafico" {
            let game = ScrabrudoGame {
                players: vec![
                    Box::new(ScrabrudoPlayer {
                        id: 0,
                        human: false,
                        hand: Hand::<Tile>{
                            items: vec![
                                Tile::A,
                                Tile::C,
                                Tile::T,
                            ],
                        },
                    }),
                    Box::new(ScrabrudoPlayer {
                        id: 1,
                        human: false,
                        hand: Hand::<Tile>{
                            items: vec![
                                Tile::B,
                                Tile::O,
                                Tile::O,
                                Tile::T,
                                Tile::S,
                                Tile::E,
                            ],
                        },
                    })
                ],
                current_index: 0,
                current_outcome: TurnOutcome::First,
            };

            // Cat is there, but has dupes
            assert!(game.is_correct(&ScrabrudoBet::from_word(&"cat".into())));
            assert!(!game.is_exactly_correct(&ScrabrudoBet::from_word(&"cat".into())));

            // Same with boot.
            assert!(game.is_correct(&ScrabrudoBet::from_word(&"boot".into())));
            assert!(!game.is_exactly_correct(&ScrabrudoBet::from_word(&"boot".into())));

            // Nonsense words are not correct.
            assert!(!game.is_correct(&ScrabrudoBet::from_word(&"ccatbo".into())));

            // A palafico word is detected as such.
            assert!(game.is_exactly_correct(&ScrabrudoBet::from_word(&"caboose".into())));
        }
    }
}
