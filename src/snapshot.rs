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

use failure::{Error, Fail};
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::path::PathBuf;

use crate::process::*;
use crate::util::*;

#[derive(Fail, Debug)]
pub enum SnapshotError {
    #[fail(
        display = "Invalid snapshot file format or unsupported version: {}",
        _0
    )]
    FormatError(String),
}

pub struct Snapshot {
    pub enabled: bool,
    pub command: String,
    pub mappings: HashSet<PathBuf>,
}

impl Snapshot {
    pub fn new_from_process(proc: &Process) -> Result<Self, Error> {
        let command = proc.get_command()?;
        let mappings = proc.get_mapped_files();

        Ok(Snapshot {
            enabled: true,
            command,
            mappings,
        })
    }

    pub fn new_from_file<T: AsRef<Path>>(path: T) -> Result<Self, Error> {
        let mut file = BufReader::new(fs::File::open(path.as_ref())?);

        let mut header = String::new();
        file.read_line(&mut header)?;

        if header.trim() != "prefault snapshot: 1.0" {
            return Err(SnapshotError::FormatError(path.as_ref().to_string_lossy().into()).into());
        }

        let mut enabled_line = String::new();
        file.read_line(&mut enabled_line)?;

        let enabled;
        if enabled_line.contains("false") {
            enabled = false
        } else {
            enabled = true
        }

        let mut command = String::new();
        file.read_line(&mut command)?;

        command = command.trim().to_string();

        let mut mappings = HashSet::new();

        for l in file.lines() {
            let value = PathBuf::from(&l?);
            mappings.insert(value);
        }

        Ok(Snapshot {
            enabled,
            command,
            mappings,
        })
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, snapshot_dir: P) -> Result<PathBuf, Error> {
        let path = PathBuf::from(format!("{}.snapshot", hash_string(&self.command)));
        let path = snapshot_dir.as_ref().join(path);

        let mut file = BufWriter::new(fs::File::create(path.clone())?);

        writeln!(file, "prefault snapshot: 1.0")?;
        writeln!(file, "enabled: {}", self.enabled)?;
        writeln!(file, "{}", self.command)?;

        for mapping in self.mappings.iter() {
            writeln!(
                file,
                "{}",
                mapping.to_str().expect("Error converting a path to utf-8")
            )?;
        }

        println!("Wrote {}", &path.display());

        Ok(path)
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn get_hash(&self) -> u64 {
        hash_string(&self.command)
    }
}
