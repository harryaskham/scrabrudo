/// Definition of a single tile.
use crate::hand::*;
use crate::testing;

use rand::distributions::Standard;
use rand::Rng;
use std::cmp::Ord;
use speculate::speculate;

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
}

impl rand::distributions::Distribution<Tile> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Tile {
        // TODO: Incorporate proper tile probabilities here.
        match rng.gen_range(0, 26) {
            0 => Tile::A,
            1 => Tile::B,
            2 => Tile::C,
            3 => Tile::D,
            4 => Tile::E,
            5 => Tile::F,
            6 => Tile::G,
            7 => Tile::H,
            8 => Tile::I,
            9 => Tile::J,
            10 => Tile::K,
            11 => Tile::L,
            12 => Tile::M,
            13 => Tile::N,
            14 => Tile::O,
            15 => Tile::P,
            16 => Tile::Q,
            17 => Tile::R,
            18 => Tile::S,
            19 => Tile::T,
            20 => Tile::U,
            21 => Tile::V,
            22 => Tile::W,
            23 => Tile::X,
            24 => Tile::Y,
            25 => Tile::Z,
            _ => panic!(),
        }
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
