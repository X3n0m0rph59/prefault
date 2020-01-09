/*
    prefault
    Copyright (c) 2019-2020 the prefault developers

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

use failure::Error;
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::path::PathBuf;

// #[derive(Fail, Debug)]
// pub enum FileListError {
//     #[fail(
//         display = "Invalid file format: {}",
//         _0
//     )]
//     FormatError(String),
// }

pub struct FileList {
    pub files: HashSet<PathBuf>,
}

impl FileList {
    pub fn new_from_file<T: AsRef<Path>>(path: T) -> Result<Self, Error> {
        let file = BufReader::new(fs::File::open(path.as_ref())?);

        // let mut header = String::new();
        // file.read_line(&mut header)?;

        let mut files = HashSet::new();

        for l in file.lines() {
            let value = PathBuf::from(&l?);
            files.insert(value);
        }

        Ok(FileList { files })
    }
}
