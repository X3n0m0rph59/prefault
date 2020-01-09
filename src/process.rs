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

use failure::{Error, Fail};
use libc;
use std::collections::HashSet;
use std::fs::{read_dir, File};
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Fail, Debug)]
pub enum ProcessError {
    #[fail(display = "Could not open process: {}", _0)]
    ReadError(#[fail(cause)] io::Error),

    #[fail(display = "Could not parse a mapping")]
    ParseMappingError(#[fail(cause)] Error),

    #[fail(display = "Could not enumerate processes")]
    EnumProcessesError(#[fail(cause)] Error),
}

// #[derive(Fail, Debug)]
// #[fail(display = "Could not parse a mapping")]
// pub struct ProcessError {}

#[derive(Debug, Clone, PartialEq)]
pub struct Mapping {
    pub file: Option<PathBuf>,
    pub start: usize,
    pub end: usize,

    pub read: bool,
    pub write: bool,
    pub exec: bool,
    pub shared: bool,
    pub private: bool,
}

impl FromStr for Mapping {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let comp: Vec<&str> = s.split(' ').collect();
        let path = s[s.rfind(' ').unwrap() + 1..].to_owned();

        let file: Option<PathBuf> = if !path.trim().is_empty() {
            Some(PathBuf::from(&path))
        } else {
            None
        };

        let adresses: Vec<&str> = comp[0].split('-').collect();
        let start: usize = usize::from_str_radix(adresses[0], 16).unwrap();
        let end: usize = usize::from_str_radix(adresses[1], 16).unwrap();

        let flags = comp[1];
        let read = flags.contains('r');
        let write = flags.contains('w');
        let exec = flags.contains('x');
        let shared = flags.contains('s');
        let private = flags.contains('p');

        Ok(Mapping {
            file,
            start,
            end,
            read,
            write,
            exec,
            shared,
            private,
        })
    }
}

pub struct ProcessIterator {
    cur: std::fs::ReadDir,
}

impl std::iter::Iterator for ProcessIterator {
    type Item = Process;

    fn next(&mut self) -> Option<Process> {
        // find the next valid `proc` directory entry
        let path = 'LOOP: loop {
            let entry = self.cur.next();
            if entry.is_none() {
                break 'LOOP None; // maybe end of `ReadDir` iterator
            }

            let entry = entry.unwrap().unwrap();
            let path = entry.path();

            let path_str = path.to_string_lossy();
            if path_str == "/proc/self"
                || path_str == "/proc/thread-self"
                || path_str == ".."
                || path_str == "."
                || !Path::join(&path, "comm").exists()
            {
                continue; // we found a known `bad` directory, skip it
            } else {
                break 'LOOP Some(path); // we found the next valid task directory
            }
        };

        if let Some(path) = path {
            let path = path.to_string_lossy();
            let pid_str = path.trim_matches(|c| !char::is_numeric(c));
            let pid: libc::pid_t = pid_str.parse::<libc::pid_t>().unwrap();

            Process::new(pid).ok()
        } else {
            None // end of iteration
        }
    }
}

#[derive(Debug)]
pub struct Process {
    pub pid: libc::pid_t,
    pub maps: Vec<Mapping>,
}

impl Process {
    pub fn new(pid: libc::pid_t) -> Result<Self, Error> {
        let path = PathBuf::from(format!("/proc/{}/maps", pid));

        match File::open(path) {
            Ok(file) => {
                let f = BufReader::new(file);

                let mut maps = vec![];
                for line in f.lines() {
                    let l = line.unwrap();

                    let mapping = Mapping::from_str(&l).map_err(ProcessError::ParseMappingError)?;
                    maps.push(mapping);
                }

                Ok(Process { pid, maps })
            }

            Err(e) => Err(ProcessError::ReadError(e).into()),
        }
    }

    pub fn enumerate() -> Result<ProcessIterator, Error> {
        Ok(read_dir("/proc/")
            .and_then(|entry| Ok(ProcessIterator { cur: entry }))
            .map_err(|e| ProcessError::EnumProcessesError(e.into()))?)
    }

    pub fn get_command(&self) -> Result<String, Error> {
        let path = PathBuf::from(format!("/proc/{}/comm", self.pid));

        match File::open(path) {
            Ok(file) => {
                let mut f = BufReader::new(file);

                let mut comm = String::new();
                f.read_to_string(&mut comm)?;

                comm = comm.trim().into();

                let v: Vec<&str> = comm.split('\u{0}').collect();

                Ok(v[0].to_owned())
            }

            Err(e) => Err(ProcessError::ReadError(e).into()),
        }
    }

    pub fn get_mapped_files(&self) -> HashSet<PathBuf> {
        let mut result = HashSet::new();

        for mapping in self.maps.iter().cloned() {
            if let Some(file) = mapping.file {
                if !is_section_mapping(&file) {
                    result.insert(file);
                }
            }
        }

        result
    }
}

fn is_section_mapping<T: AsRef<Path>>(mapping: T) -> bool {
    let mapping = mapping.as_ref().to_str().unwrap();

    if mapping == "[vdso]" {
        return true;
    }

    if mapping == "[stack]" {
        return true;
    }

    if mapping == "[heap]" {
        return true;
    }

    if mapping == "[vsyscall]" {
        return true;
    }

    if mapping == "[vvar]" {
        return true;
    }

    if mapping.contains("(deleted)") {
        return true;
    }

    if mapping.trim().is_empty() {
        return true;
    }

    false
}
