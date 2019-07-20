use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn hash_string<T: Hash>(s: T) -> u64 {
    let mut hasher = DefaultHasher::new();

    s.hash(&mut hasher);
    hasher.finish()
}
