use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use pretty_bytes::converter::convert;

pub fn hash_string<T: Hash>(s: T) -> u64 {
    let mut hasher = DefaultHasher::new();

    s.hash(&mut hasher);
    hasher.finish()
}

pub fn format_filesize(size: u64) -> String {
    format!("{}", convert(size as f64))
}
