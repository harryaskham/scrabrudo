/// Definition of a single tile.

use crate::hand::*;

use rand::distributions::Standard;
use rand::Rng;
use std::cmp::Ord;

// TODO: Extended alphabets, wildcards
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Tile {
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, Blank
}

impl Holdable for Tile {
    fn get_random() -> Self {
        rand::random()
    }
}

impl Tile {
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
            Tile::Blank => ' ',
        }
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
            Tile::Blank,
        ]
    }
}

impl rand::distributions::Distribution<Tile> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Tile {
        // TODO: Incorporate proper tile probabilities here.
        match rng.gen_range(0, 27) {
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
            26 => Tile::Blank,
            _ => panic!(),
        }
    }
}
