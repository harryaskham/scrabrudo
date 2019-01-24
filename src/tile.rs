/// Definition of a single tile.
use crate::hand::*;
use crate::testing;

use rand::distributions::Standard;
use rand::Rng;
use speculate::speculate;
use std::cmp::Ord;

// TODO: Extended alphabets, wildcards
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Tile {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
}

impl Holdable for Tile {
    fn get_random() -> Self {
        rand::random()
    }
}

impl Tile {
    pub fn from_char(c: char) -> Self {
        match c {
            'a' => Tile::A,
            'b' => Tile::B,
            'c' => Tile::C,
            'd' => Tile::D,
            'e' => Tile::E,
            'f' => Tile::F,
            'g' => Tile::G,
            'h' => Tile::H,
            'i' => Tile::I,
            'j' => Tile::J,
            'k' => Tile::K,
            'l' => Tile::L,
            'm' => Tile::M,
            'n' => Tile::N,
            'o' => Tile::O,
            'p' => Tile::P,
            'q' => Tile::Q,
            'r' => Tile::R,
            's' => Tile::S,
            't' => Tile::T,
            'u' => Tile::U,
            'v' => Tile::V,
            'w' => Tile::W,
            'x' => Tile::X,
            'y' => Tile::Y,
            'z' => Tile::Z,
            _ => panic!(),
        }
    }

    pub fn char(&self) -> char {
        match &self {
            Tile::A => 'a',
            Tile::B => 'b',
            Tile::C => 'c',
            Tile::D => 'd',
            Tile::E => 'e',
            Tile::F => 'f',
            Tile::G => 'g',
            Tile::H => 'h',
            Tile::I => 'i',
            Tile::J => 'j',
            Tile::K => 'k',
            Tile::L => 'l',
            Tile::M => 'm',
            Tile::N => 'n',
            Tile::O => 'o',
            Tile::P => 'p',
            Tile::Q => 'q',
            Tile::R => 'r',
            Tile::S => 's',
            Tile::T => 't',
            Tile::U => 'u',
            Tile::V => 'v',
            Tile::W => 'w',
            Tile::X => 'x',
            Tile::Y => 'y',
            Tile::Z => 'z',
        }
    }

    pub fn as_usize(&self) -> usize {
        (self.char() as u32 - 'a' as u32) as usize
    }

    pub fn from_usize(u: usize) -> Tile {
        Tile::from_char((u as u8 + 'a' as u8) as char)
    }

    pub fn all() -> Vec<Tile> {
        vec![
            Tile::A,
            Tile::B,
            Tile::C,
            Tile::D,
            Tile::E,
            Tile::F,
            Tile::G,
            Tile::H,
            Tile::I,
            Tile::J,
            Tile::K,
            Tile::L,
            Tile::M,
            Tile::N,
            Tile::O,
            Tile::P,
            Tile::Q,
            Tile::R,
            Tile::S,
            Tile::T,
            Tile::U,
            Tile::V,
            Tile::W,
            Tile::X,
            Tile::Y,
            Tile::Z,
        ]
    }

    pub fn score(&self) -> u32 {
        match &self {
            Tile::A => 1,
            Tile::B => 3,
            Tile::C => 3,
            Tile::D => 2,
            Tile::E => 1,
            Tile::F => 4,
            Tile::G => 2,
            Tile::H => 4,
            Tile::I => 1,
            Tile::J => 8,
            Tile::K => 5,
            Tile::L => 1,
            Tile::M => 3,
            Tile::N => 1,
            Tile::O => 1,
            Tile::P => 3,
            Tile::Q => 10,
            Tile::R => 1,
            Tile::S => 1,
            Tile::T => 1,
            Tile::U => 1,
            Tile::V => 4,
            Tile::W => 4,
            Tile::X => 8,
            Tile::Y => 4,
            Tile::Z => 10,
        }
    }
}

/* Copied from Wiki for UK Scrabble distribution:
1 point: E ×12, A ×9, I ×9, O ×8, N ×6, R ×6, T ×6, L ×4, S ×4, U ×4
2 points: D ×4, G ×3
3 points: B ×2, C ×2, M ×2, P ×2
4 points: F ×2, H ×2, V ×2, W ×2, Y ×2
5 points: K ×1
8 points: J ×1, X ×1
10 points: Q ×1, Z ×1

By hand, that's
[9, 2, 2, 4, 12, 2, 3, 2, 9, 1, 1, 4, 2, 6, 8, 2, 1, 6, 4, 6, 4, 2, 2, 1, 2, 1]
*/

impl rand::distributions::Distribution<Tile> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Tile {
        // TODO: This could be a lot more efficient. We compute the CDF every time.
        let mut distribution: Vec<u32> = vec![
            9, 2, 2, 4, 12, 2, 3, 2, 9, 1, 1, 4, 2, 6, 8, 2, 1, 6, 4, 6, 4, 2, 2, 1, 2, 1,
        ];
        for i in 1..distribution.len() {
            distribution[i] += distribution[i - 1]
        }

        let bound = rng.gen_range(0, distribution.last().unwrap());
        for i in 0..distribution.len() {
            if distribution[i] >= bound {
                return Tile::from_usize(i);
            }
        }
        panic!("Should not reach here, we covered every case above");
    }
}

speculate! {
    before {
        testing::set_up();
    }

    describe "tile" {
        it "represents tiles as usize" {
            assert_eq!(0, Tile::A.as_usize());
            assert_eq!(25, Tile::Z.as_usize());
        }

        it "creates tiles from usize" {
            assert_eq!(Tile::A, Tile::from_usize(0));
            assert_eq!(Tile::Z, Tile::from_usize(25));
        }
    }
}
