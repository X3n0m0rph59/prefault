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

use config;
use ctrlc;
use failure::{Error, Fail};
use lazy_static::lazy_static;
use libc;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use structopt::StructOpt;
use walkdir;

mod filelist;
mod memory;
mod process;
mod snapshot;
mod util;

// use crate::memory::*;
use crate::filelist::*;
use crate::process::*;
use crate::snapshot::*;
// use crate::util::*;

lazy_static! {
    pub static ref RUNNING: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Pre-fault and optionally lock files into the kernel's page cache memory")]
pub struct Options {
    #[structopt(short = "c", help = "Specify an alternative configuration file")]
    config_file: Option<PathBuf>,

    #[structopt(
        short = "v",
        parse(from_occurrences),
        help = "Specify verbosity of log output"
    )]
    verbosity: u8,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(
        name = "list",
        about = "List available process snapshots and static file lists"
    )]
    List {
        #[structopt(short = "f", long = "filter")]
        filter: Option<String>,
    },

    #[structopt(name = "enable", about = "Enable loading of process snapshots")]
    Enable {
        #[structopt(short = "f", long = "filter")]
        filter: Option<String>,
    },

    #[structopt(name = "disable", about = "Disable loading of process snapshots")]
    Disable {
        #[structopt(short = "f", long = "filter")]
        filter: Option<String>,
    },

    #[structopt(name = "show", about = "Show information about process snapshots")]
    Show {
        #[structopt(short = "f", long = "filter")]
        filter: Option<String>,
    },

    #[structopt(name = "snapshot", about = "Take snapshots of running processes")]
    Snapshot {
        #[structopt(short = "f", long = "filter")]
        filter: Option<String>,

        #[structopt(short = "p")]
        pid: Option<libc::pid_t>,
    },

    #[structopt(
        name = "incore",
        about = "Show which files of a process snapshot are resident in the page cache"
    )]
    Incore {
        #[structopt(short = "f", long = "filter")]
        filter: Option<String>,

        #[structopt(short = "p")]
        pid: Option<libc::pid_t>,
    },

    //#[structopt(name = "trace", about = "Trace a process and record accessed files")]
    //Trace {
    //#[structopt(short = "c")]
    //command: Option<String>,
    //},

    #[structopt(name = "remove", about = "Remove a process snapshot")]
    Remove {
        #[structopt(short = "f", long = "filter")]
        filter: Option<String>,
    },

    #[structopt(name = "cache", about = "Fault in files from process snapshots")]
    Cache {
        #[structopt(short = "f", long = "filter")]
        filter: Option<String>,
    },

    #[structopt(
        name = "mlock",
        about = "Lock files from process snapshots into memory"
    )]
    Mlock {
        #[structopt(short = "f", long = "filter")]
        filter: Option<String>,
    },
}

#[derive(Fail, Debug)]
#[fail(display = "An error occurred")]
enum CommandError {
    #[fail(display = "Invalid command parameters: {}", _0)]
    InvalidParamaters(String),

    // #[fail(display = "Invalid filter expression")]
    // InvalidFilter,
    #[fail(display = "Error during command execution: {}", _0)]
    ExecutionError(#[fail(cause)] Error),

    #[fail(display = "Could not read process information: {}", msg)]
    Process { msg: String },
}

fn do_list<T: AsRef<str>, P: AsRef<Path>>(
    filter: Option<T>,
    static_filelist_dir: P,
    snapshot_dir: P,
    opts: &Options,
) -> Result<(), Error> {
    if filter.is_none() {
        println!("{}:", static_filelist_dir.as_ref().display());

        for entry in walkdir::WalkDir::new(static_filelist_dir.as_ref()) {
            let p = entry?;
            if p.file_type().is_dir()
                || p.path().extension().unwrap_or_else(|| OsStr::new("")) != "list"
            {
                continue;
            }

            let filelist =
                FileList::new_from_file(p.path()).map_err(CommandError::ExecutionError)?;

            let mut total_size = 0;
            for file in filelist.files.iter() {
                match fs::metadata(&file) {
                    Ok(metadata) => {
                        let size = metadata.len();
                        total_size += size;
                    }

                    Err(e) => eprintln!("{}: {}", &file.display(), e),
                }
            }

            println!(
                "{} ({} files, {})",
                p.file_name().to_string_lossy(),
                filelist.files.len(),
                util::format_file_size(total_size)
            );
            println!();
        }
    }

    println!("{}:", snapshot_dir.as_ref().display());

    for entry in walkdir::WalkDir::new(snapshot_dir.as_ref()) {
        let p = entry?;
        if p.file_type().is_dir() || !match_filter(filter.as_ref(), &p.path(), &opts) {
            continue;
        }

        let snapshot = Snapshot::new_from_file(p.path()).map_err(CommandError::ExecutionError)?;

        let mut total_size = 0;
        for mapping in snapshot.mappings.iter() {
            match fs::metadata(&mapping) {
                Ok(metadata) => {
                    let size = metadata.len();
                    total_size += size;
                }

                Err(e) => eprintln!("{}: {}", &mapping.display(), e),
            }
        }

        println!(
            "{} {} ({} files, {})",
            snapshot.get_hash(),
            &snapshot.command,
            snapshot.mappings.len(),
            util::format_file_size(total_size)
        );
    }

    Ok(())
}

fn do_set_state<T: AsRef<str>, P: AsRef<Path>>(
    filter: Option<T>,
    snapshot_dir: P,
    enable: bool,
    opts: &Options,
) -> Result<(), Error> {
    println!("{}:", snapshot_dir.as_ref().display());

    for entry in walkdir::WalkDir::new(snapshot_dir.as_ref()) {
        let p = entry?;
        if p.file_type().is_dir() || !match_filter(filter.as_ref(), &p.path(), &opts) {
            continue;
        }

        let mut snapshot =
            Snapshot::new_from_file(p.path()).map_err(CommandError::ExecutionError)?;

        snapshot.set_enabled(enable);
        snapshot.save_to_file(snapshot_dir.as_ref())?;

        println!(
            "{} ({} files) - Enabled: {}",
            snapshot.command,
            snapshot.mappings.len(),
            enable
        );
    }

    Ok(())
}

fn do_show<T: AsRef<str>, P: AsRef<Path>>(
    filter: Option<T>,
    snapshot_dir: P,
    opts: &Options,
) -> Result<(), Error> {
    for entry in walkdir::WalkDir::new(snapshot_dir.as_ref()) {
        let p = entry?;
        if p.file_type().is_dir() || !match_filter(filter.as_ref(), &p.path(), &opts) {
            continue;
        }

        let snapshot = Snapshot::new_from_file(p.path()).map_err(CommandError::ExecutionError)?;

        println!(
            "{} ({} files) - Enabled: {}",
            snapshot.command,
            snapshot.mappings.len(),
            snapshot.enabled
        );

        let mut total_size = 0;
        for mapping in snapshot.mappings {
            match fs::metadata(&mapping) {
                Ok(metadata) => {
                    let size = metadata.len();
                    total_size += size;

                    println!(
                        "\t{} ({})",
                        &mapping.display(),
                        util::format_file_size(size)
                    );
                }

                Err(e) => eprintln!("{}: {}", &mapping.display(), e),
            }
        }

        println!("Total: {}", util::format_file_size(total_size));
        println!();
    }

    Ok(())
}

fn do_snapshot<T: AsRef<str>, P: AsRef<Path>>(
    filter: Option<T>,
    pid: Option<libc::pid_t>,
    snapshot_dir: P,
    opts: &Options,
) -> Result<(), CommandError> {
    if filter.is_none() && pid.is_none() {
        return Err(CommandError::InvalidParamaters(
            "Neither filter nor PID specified".into(),
        ));
    }

    if let Some(pid) = pid {
        match Process::new(pid) {
            Ok(proc) => match Snapshot::new_from_process(&proc) {
                Ok(snapshot) => {
                    let path = snapshot
                        .save_to_file(snapshot_dir.as_ref())
                        .map_err(CommandError::ExecutionError)?;

                    println!("Wrote {}", &path.display());
                }

                Err(e) => return Err(CommandError::ExecutionError(e)),
            },

            Err(e) => {
                return Err(CommandError::Process {
                    msg: format!("{}", e),
                })
            }
        }
    } else if let Some(filter) = filter {
        for process in Process::enumerate().map_err(CommandError::ExecutionError)? {
            if !match_filter_process(Some(filter.as_ref()), &process, &opts) {
                continue;
            }

            match Snapshot::new_from_process(&process).map_err(CommandError::ExecutionError) {
                Ok(snapshot) => {
                    let path = snapshot
                        .save_to_file(snapshot_dir.as_ref())
                        .map_err(CommandError::ExecutionError)?;

                    println!("Wrote {}", &path.display());
                }

                Err(e) => return Err(CommandError::ExecutionError(e.into())),
            }
        }
    } else {
        return Err(CommandError::InvalidParamaters(
            "Neither filter nor PID specified".into(),
        ));
    }

    Ok(())
}

fn do_incore<T: AsRef<str>, P: AsRef<Path>>(
    filter: Option<T>,
    pid: Option<libc::pid_t>,
    snapshot_dir: P,
    opts: &Options,
) -> Result<(), Error> {
    if filter.is_none() && pid.is_none() {
        return Err(
            CommandError::InvalidParamaters("Neither filter nor PID specified".into()).into(),
        );
    }

    if let Some(pid) = pid {
        match Process::new(pid) {
            Ok(proc) => {
                println!("{} mappings:", proc.get_command()?);
                match Snapshot::new_from_process(&proc) {
                    Ok(snapshot) => {
                        let paths: Vec<PathBuf> = snapshot.mappings.iter().cloned().collect();
                        memory::print_fincore(&paths)
                            .map_err(|e| CommandError::ExecutionError(e.into()))?;
                    }

                    Err(e) => return Err(CommandError::ExecutionError(e).into()),
                }
            }

            Err(e) => {
                return Err(CommandError::Process {
                    msg: format!("{}", e),
                }
                .into())
            }
        }
    } else if let Some(filter) = filter {
        for entry in walkdir::WalkDir::new(snapshot_dir.as_ref()) {
            let p = entry?;
            if p.file_type().is_dir() || !match_filter(Some(filter.as_ref()), &p.path(), &opts) {
                continue;
            }

            match Snapshot::new_from_file(p.path()).map_err(CommandError::ExecutionError) {
                Ok(snapshot) => {
                    let paths: Vec<PathBuf> = snapshot.mappings.iter().cloned().collect();
                    memory::print_fincore(&paths)
                        .map_err(|e| CommandError::ExecutionError(e.into()))?;
                }

                Err(e) => return Err(CommandError::ExecutionError(e.into()).into()),
            }
        }
    } else {
        return Err(
            CommandError::InvalidParamaters("Neither filter nor PID specified".into()).into(),
        );
    }

    Ok(())
}

fn do_remove<T: AsRef<str>, P: AsRef<Path>>(
    filter: Option<T>,
    snapshot_dir: P,
    opts: &Options,
) -> Result<(), Error> {
    // Err(CommandError::InvalidFilter)

    for entry in walkdir::WalkDir::new(snapshot_dir.as_ref()) {
        let p = entry?;
        if p.file_type().is_dir() || !match_filter(filter.as_ref(), &p.path(), &opts) {
            continue;
        }

        if p.path().extension().unwrap_or_else(|| OsStr::new("")) == "snapshot" {
            println!("Removing {}", p.path().display());
            fs::remove_file(p.path()).map_err(|e| CommandError::ExecutionError(e.into()))?;
        }
    }

    Ok(())
}

fn do_cache<T: AsRef<str>, P: AsRef<Path>>(
    filter: Option<T>,
    static_filelist_dir: P,
    snapshot_dir: P,
    opts: &Options,
) -> Result<(), Error> {
    for entry in walkdir::WalkDir::new(static_filelist_dir.as_ref()) {
        let p = entry?;
        if p.file_type().is_dir()
            || p.path().extension().unwrap_or_else(|| OsStr::new("")) != "list"
        {
            continue;
        }

        let filelist = FileList::new_from_file(p.path()).map_err(CommandError::ExecutionError)?;

        let files: Vec<PathBuf> = filelist.files.iter().cloned().collect();

        memory::prime_dentry_cache(&files);
        memory::prefault_file_mappings(&files)
            .map_err(|e| CommandError::ExecutionError(e.into()))?;
    }

    for entry in walkdir::WalkDir::new(snapshot_dir.as_ref()) {
        let p = entry?;
        if p.file_type().is_dir() || !match_filter(filter.as_ref(), &p.path(), &opts) {
            continue;
        }

        let snapshot = Snapshot::new_from_file(p.path()).map_err(CommandError::ExecutionError)?;

        if snapshot.enabled {
            if opts.verbosity > 0 {
                println!("{}", snapshot.command);
            }

            let files: Vec<PathBuf> = snapshot.mappings.iter().cloned().collect();

            memory::prime_dentry_cache(&files);
            memory::prefault_file_mappings(&files)
                .map_err(|e| CommandError::ExecutionError(e.into()))?;
        }
    }

    Ok(())
}

fn do_mlock<T: AsRef<str>, P: AsRef<Path>>(
    filter: Option<T>,
    static_filelist_dir: P,
    snapshot_dir: P,
    opts: &Options,
) -> Result<(), Error> {
    for entry in walkdir::WalkDir::new(static_filelist_dir.as_ref()) {
        let p = entry?;
        if p.file_type().is_dir()
            || p.path().extension().unwrap_or_else(|| OsStr::new("")) != "list"
        {
            continue;
        }

        let filelist = FileList::new_from_file(p.path()).map_err(CommandError::ExecutionError)?;

        let files: Vec<PathBuf> = filelist.files.iter().cloned().collect();

        // memory::prime_dentry_cache(&files);
        memory::mlock_file_mappings(&files, opts)
            .map_err(|e| CommandError::ExecutionError(e.into()))?;
    }

    for entry in walkdir::WalkDir::new(snapshot_dir.as_ref()) {
        let p = entry?;
        if p.file_type().is_dir() || !match_filter(filter.as_ref(), &p.path(), &opts) {
            continue;
        }

        let snapshot = Snapshot::new_from_file(p.path()).map_err(CommandError::ExecutionError)?;

        if snapshot.enabled {
            if opts.verbosity > 0 {
                println!("{}", snapshot.command);
            }

            let files: Vec<PathBuf> = snapshot.mappings.iter().cloned().collect();

            // memory::prime_dentry_cache(&files);
            memory::mlock_file_mappings(&files, opts)
                .map_err(|e| CommandError::ExecutionError(e.into()))?;
        }
    }

    Ok(())
}

fn match_filter<T: AsRef<str>, P: AsRef<Path>>(
    filter: Option<T>,
    snapshot: P,
    _opts: &Options,
) -> bool {
    // no filter matches all
    if filter.is_none() {
        return true;
    }

    let filter = filter.unwrap();
    let params: Vec<&str> = filter.as_ref().split('=').collect();

    if params.len() != 2 {
        panic!("WARNING: Invalid filter syntax: '{}'", &filter.as_ref());
        // return false;
    }

    if params[0].starts_with("comm") {
        match snapshot::Snapshot::new_from_file(snapshot.as_ref()) {
            Ok(snapshot) => {
                // TODO: Add support for regex
                snapshot.command.contains(params[1].trim())
            }

            Err(e) => {
                eprintln!("WARNING: Error matching filter: {}", e);
                false
            }
        }
    } else if params[0].starts_with("hash") {
        match snapshot::Snapshot::new_from_file(snapshot.as_ref()) {
            Ok(snapshot) => match params[1].parse::<u64>() {
                Ok(hash) => snapshot.get_hash() == hash,

                Err(e) => {
                    panic!("Invalid hash value specified: {}", e);
                    // return false;
                }
            },

            Err(e) => {
                eprintln!("WARNING: Error matching filter: {}", e);
                false
            }
        }
    } else {
        false
    }
}

fn match_filter_process<T: AsRef<str>>(
    filter: Option<T>,
    process: &Process,
    _opts: &Options,
) -> bool {
    // no filter matches all
    if filter.is_none() {
        return true;
    }

    let filter = filter.unwrap();
    let params: Vec<&str> = filter.as_ref().split('=').collect();

    if params.len() != 2 {
        panic!("WARNING: Invalid filter syntax: '{}'", &filter.as_ref());
        // return false;
    }

    params[0].starts_with("comm") && process.get_command().unwrap().starts_with(params[1].trim())
}

fn main() {
    let r = RUNNING.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let opts = Options::from_args();
    let mut settings = config::Config::default();

    settings
        .merge(config::File::new(
            opts.config_file
                .clone()
                .unwrap_or_else(|| "/etc/prefault/prefault.conf".into())
                .to_str()
                .unwrap(),
            config::FileFormat::Toml,
        ))
        .expect("Could not read configuration file");

    let home_dir = PathBuf::from(env::var("HOME").unwrap_or_else(|_| "/root".into()));
    let mut snapshot_dir = settings
        .get::<PathBuf>("snapshot_dir")
        .unwrap_or_else(|_| PathBuf::from("/var/lib/prefault/snapshots"));

    if snapshot_dir.starts_with("~/") {
        snapshot_dir = home_dir.join(
            snapshot_dir
                .strip_prefix(Path::new("~/"))
                .expect("Error building snapshot_dir"),
        )
    }

    // println!("{}", &snapshot_dir.display());
    fs::create_dir_all(&snapshot_dir).expect("Error creating snapshot directory");

    let mut static_filelist_dir = settings
        .get::<PathBuf>("static_filelist_dir")
        .unwrap_or_else(|_| PathBuf::from("/etc/prefault/cache.d"));

    if static_filelist_dir.starts_with("~/") {
        static_filelist_dir = home_dir.join(
            static_filelist_dir
                .strip_prefix(Path::new("~/"))
                .expect("Error building static_filelist_dir"),
        )
    }

    match opts.cmd {
        Command::List { ref filter, .. } => {
            do_list(filter.as_ref(), &static_filelist_dir, &snapshot_dir, &opts)
                .unwrap_or_else(|e| eprintln!("{}", e))
        }

        Command::Enable { ref filter, .. } => {
            do_set_state(filter.as_ref(), snapshot_dir, true, &opts)
                .unwrap_or_else(|e| eprintln!("{}", e))
        }

        Command::Disable { ref filter, .. } => {
            do_set_state(filter.as_ref(), snapshot_dir, false, &opts)
                .unwrap_or_else(|e| eprintln!("{}", e))
        }

        Command::Show { ref filter, .. } => {
            do_show(filter.as_ref(), snapshot_dir, &opts).unwrap_or_else(|e| eprintln!("{}", e))
        }

        Command::Snapshot {
            ref filter, pid, ..
        } => {
            do_snapshot(filter.as_ref(), pid, snapshot_dir, &opts)
                .unwrap_or_else(|e| eprintln!("{}", e));
        }

        Command::Incore {
            ref filter, pid, ..
        } => do_incore(filter.as_ref(), pid, snapshot_dir, &opts)
            .unwrap_or_else(|e| eprintln!("{}", e)),

        //Command::Trace { command: _, .. } => {
        //println!("Trace subcommand is currently not implemented");
        //}

        Command::Remove { ref filter, .. } => {
            do_remove(filter.as_ref(), snapshot_dir, &opts).unwrap_or_else(|e| eprintln!("{}", e))
        }

        Command::Cache { ref filter, .. } => {
            do_cache(filter.as_ref(), &static_filelist_dir, &snapshot_dir, &opts)
                .unwrap_or_else(|e| eprintln!("{}", e))
        }

        Command::Mlock { ref filter, .. } => {
            do_mlock(filter.as_ref(), &static_filelist_dir, &snapshot_dir, &opts)
                .unwrap_or_else(|e| eprintln!("{}", e));
            println!("Going to sleep now");

            if unsafe { libc::isatty(0) == 1 } {
                loop {
                    thread::sleep(Duration::from_millis(1000));

                    if !RUNNING.load(Ordering::SeqCst) {
                        break;
                    }
                }
            } else {
                thread::sleep(Duration::from_millis(std::u64::MAX));
            }

            println!("Exiting");
        }
    }
}
