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

pub mod game;
pub mod player;
pub mod hand;
pub mod bet;

use crate::game::*;
use crate::bet::*;
use crate::hand::*;
use crate::player::*;

use speculate::speculate;
use std::collections::HashSet;
use std::env;

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
            let bet_3 = bet(DieVal::Two, 3);
            let bet_4 = bet(DieVal::Three, 3);
            let bet_5 = bet(DieVal::Three, 4);
            let bet_6 = bet(DieVal::Two, 5);
            let bet_7 = bet(DieVal::Two, 6);
            let bet_8 = bet(DieVal::Three, 8);
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
