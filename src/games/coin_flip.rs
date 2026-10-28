use rand::{RngExt, rng};

fn play_coin_flip() -> String {
    if rng().random_bool(0.5) {
        "heads".to_string()
    } else {
        "tails".to_string()
    }
}