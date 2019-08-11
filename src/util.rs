/*
    prefault
    Copyright (C) 2019 the prefault developers

    This file is part of prefault.

    Prefault is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Prefault is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with Prefault.  If not, see <http://www.gnu.org/licenses/>.
*/

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use pretty_bytes::converter::convert;

pub fn hash_string<T: Hash>(s: T) -> u64 {
    let mut hasher = DefaultHasher::new();

    s.hash(&mut hasher);
    hasher.finish()
}

pub fn format_filesize(size: u64) -> String {
    convert(size as f64)
}
