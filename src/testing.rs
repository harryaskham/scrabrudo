/// Test utils.

use crate::dict;
use std::sync::Mutex;

lazy_static! {
    pub static ref SET_UP_DONE: Mutex<bool> = Mutex::new(false);
}

pub fn set_up() {
    let mut state = SET_UP_DONE.lock().unwrap();
    if !*state {
        pretty_env_logger::try_init();
        dict::init_dict("data/google-10000-english.txt");
        dict::init_lookup("data/simple_5_100.sstable");
        *state = true;
    }
}
