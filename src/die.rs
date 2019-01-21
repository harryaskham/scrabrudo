/// Definition of a single tile.

use crate::hand::*;

use rand::distributions::Standard;
use rand::Rng;
use std::cmp::Ord;

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Die {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
}

impl Holdable for Die {
    fn get_random() -> Self {
        rand::random()
    }
}

impl Die {
    pub fn int(&self) -> u32 {
        match &self {
            Die::One => 1,
            Die::Two => 2,
            Die::Three => 3,
            Die::Four => 4,
            Die::Five => 5,
            Die::Six => 6,
        }
    }

    pub fn all() -> Vec<Die> {
        vec![
            Die::One,
            Die::Two,
            Die::Three,
            Die::Four,
            Die::Five,
            Die::Six,
        ]
    }

    pub fn from_usize(x: usize) -> Die {
        let all = Die::all();
        all[x - 1].clone()
    }
}

// Make it possible to generate random Dies.
impl rand::distributions::Distribution<Die> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Die {
        match rng.gen_range(1, 7) {
            1 => Die::One,
            2 => Die::Two,
            3 => Die::Three,
            4 => Die::Four,
            5 => Die::Five,
            6 => Die::Six,
            _ => panic!(),
        }
    }
}
