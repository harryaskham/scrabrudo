/// Test utils.

use crate::dict::*;
use std::sync::Mutex;

lazy_static! {
    pub static ref SET_UP_DONE: Mutex<bool> = Mutex::new(false);
}

pub fn set_up() {
    let mut state = SET_UP_DONE.lock().unwrap();
    if !*state {
        pretty_env_logger::try_init();
        SCRABBLE_DICT.lock().unwrap().init_dict("data/scrabble.txt");
        SCRABBLE_DICT.lock().unwrap().init_lookup("data/lookup_5_1000.bin");

        *state = true;
    }
}
