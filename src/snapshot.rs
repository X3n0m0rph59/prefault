use failure::Error;
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::path::PathBuf;

use crate::process::*;
use crate::util::*;

pub struct Snapshot {
    pub command: String,
    pub mappings: HashSet<PathBuf>,
}

impl Snapshot {
    pub fn new_from_process(proc: &Process) -> Result<Self, Error> {
        let command = proc.get_command()?;
        let mappings = proc.get_mapped_files();

        Ok(Snapshot { command, mappings })
    }

    pub fn new_from_file<T: AsRef<Path>>(path: T) -> Result<Self, Error> {
        let mut file = BufReader::new(fs::File::open(path)?);

        let mut command = String::new();
        file.read_line(&mut command)?;

        let mut mappings = HashSet::new();

        for l in file.lines() {
            let value = PathBuf::from(&l?);
            mappings.insert(value);
        }

        Ok(Snapshot { command, mappings })
    }

    pub fn save_to_file(&self) -> Result<PathBuf, Error> {
        let path = PathBuf::from(format!("{}.snapshot", hash_string(&self.command)));
        let mut file = BufWriter::new(fs::File::create(path.clone())?);

        writeln!(file, "{}", self.command)?;

        for mapping in self.mappings.iter() {
            writeln!(
                file,
                "{}",
                mapping.to_str().expect("Error converting a path to utf-8")
            )?;
        }

        Ok(path)
    }
}
