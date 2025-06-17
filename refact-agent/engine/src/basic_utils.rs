use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

pub fn generate_random_hash(length: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

