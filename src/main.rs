use config;
use ctrlc;
use failure::{Error, Fail};
use lazy_static::lazy_static;
use libc;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::u64;
use structopt::StructOpt;

mod memory;
mod process;
mod snapshot;
mod util;

// use crate::memory::*;
use crate::process::*;
use crate::snapshot::*;
// use crate::util::*;

lazy_static! {
    pub static ref RUNNING: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Pre-fault and optionally lock files into the kernel's page cache")]
struct Options {
    #[structopt(short = "c", help = "Specify a userfault configuration file")]
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
    #[structopt(name = "list", about = "List process snapshots")]
    List {
        #[structopt(short = "f")]
        filter: Option<String>,
    },

    #[structopt(name = "enable", about = "Enable loading of a process snapshot")]
    Enable {
        #[structopt(short = "f")]
        filter: Option<String>,
    },

    #[structopt(name = "disable", about = "Disable loading of a process snapshot")]
    Disable {
        #[structopt(short = "f")]
        filter: Option<String>,
    },

    #[structopt(
        name = "show",
        about = "Show information about a process snapshot"
    )]
    Show {
        #[structopt(short = "f")]
        filter: Option<String>,
    },

    #[structopt(
        name = "snapshot",
        about = "Take snapshots of running processes",
        // help = "Examines running processes and saves their mapped files as snapshots"
    )]
    Snapshot {
        #[structopt(short = "f")]
        filter: Option<String>,

        #[structopt(short = "p")]
        pid: Option<libc::pid_t>,
    },

    #[structopt(
        name = "incore",
        about = "Show which files of a process snapshot are resident in the page cache",
    )]
    Incore {
        #[structopt(short = "f")]
        filter: Option<String>,

        #[structopt(short = "p")]
        pid: Option<libc::pid_t>,
    },

    #[structopt(name = "trace", about = "Trace a process and record accessed files")]
    Trace {
        #[structopt(short = "c")]
        command: Option<String>,
    },

    #[structopt(name = "remove", about = "Remove a process snapshot")]
    Remove {
        #[structopt(short = "f")]
        filter: Option<String>,
    },

    #[structopt(name = "cache", about = "Fault in files from process snapshots")]
    Cache {
        #[structopt(short = "f")]
        filter: Option<String>,
    },

    #[structopt(
        name = "mlock",
        about = "Lock files from process snapshots into memory"
    )]
    Mlock {
        #[structopt(short = "f")]
        filter: Option<String>,
    },
}

#[derive(Fail, Debug)]
#[fail(display = "An error occurred")]
enum CommandError {
    #[fail(display = "Invalid command parameters: {}", _0)]
    InvalidParamaters(String),

    #[fail(display = "Invalid filter expression")]
    InvalidFilter,

    #[fail(display = "Error during command execution: {}", _0)]
    ExecutionError(#[fail(cause)] Error),

    #[fail(display = "Could not read process information: {}", msg)]
    Process { msg: String },
}

fn do_snapshot<T: AsRef<str>>(
    filter: Option<T>,
    pid: Option<libc::pid_t>,
) -> Result<Vec<PathBuf>, CommandError> {
    let mut result = vec![];

    if filter.is_none() && pid.is_none() {
        return Err(CommandError::InvalidParamaters(
            "Neither filter nor pid specified".into(),
        ));
    }

    match Process::new(pid.unwrap()) {
        Ok(proc) => match Snapshot::new_from_process(&proc) {
            Ok(snapshot) => {
                let path = snapshot
                    .save_to_file()
                    .map_err(|e| CommandError::ExecutionError(e.into()))?;

                result.push(path);
            }

            Err(e) => return Err(CommandError::ExecutionError(e)),
        },

        Err(e) => {
            return Err(CommandError::Process {
                msg: format!("{}", e),
            })
        }
    }

    Ok(result)
}

fn do_mincore<T: AsRef<str>>(
    filter: Option<T>,
    pid: Option<libc::pid_t>,
) -> Result<(), CommandError> {
    if filter.is_none() && pid.is_none() {
        return Err(CommandError::InvalidParamaters(
            "Neither filter nor pid specified".into(),
        ));
    }

    match Process::new(pid.unwrap()) {
        Ok(proc) => match Snapshot::new_from_process(&proc) {
            Ok(snapshot) => {
                let paths: Vec<PathBuf> = snapshot.mappings.iter().cloned().collect();
                memory::fincore(&paths).map_err(|e| CommandError::ExecutionError(e.into()))?;
            }

            Err(e) => return Err(CommandError::ExecutionError(e)),
        },

        Err(e) => {
            return Err(CommandError::Process {
                msg: format!("{}", e),
            })
        }
    }

    Ok(())
}

fn do_cache<T: AsRef<str>>(_filter: Option<T>) -> Result<(), CommandError> {
    // Err(CommandError::InvalidFilter)

    let p = PathBuf::from("123.snapshot");
    let snapshot = Snapshot::new_from_file(p).map_err(|e| CommandError::ExecutionError(e))?;

    let files: Vec<PathBuf> = snapshot.mappings.iter().cloned().collect();

    memory::prime_dentry_cache(&files);
    memory::prefault_file_mappings(&files).map_err(|e| CommandError::ExecutionError(e.into()))?;

    Ok(())
}

fn do_mlock<T: AsRef<str>, P: AsRef<Path>>(
    _filter: Option<T>,
    snapshot_dir: P,
) -> Result<(), CommandError> {
    // Err(CommandError::InvalidFilter)

    let p = PathBuf::from("123.snapshot");
    let snapshot = Snapshot::new_from_file(p).map_err(|e| CommandError::ExecutionError(e))?;

    let files: Vec<PathBuf> = snapshot.mappings.iter().cloned().collect();

    memory::prime_dentry_cache(&files);
    memory::mlock_file_mappings(&files).map_err(|e| CommandError::ExecutionError(e.into()))?;

    Ok(())
}

fn main() {
    let r = RUNNING.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let opts = Options::from_args();
    let mut settings = config::Config::default();

    if opts.config_file.is_some() {
        settings
            .merge(config::File::new(
                opts.config_file.unwrap().to_str().unwrap(),
                config::FileFormat::Toml,
            ))
            .expect("Could not read configuration file");
    }

    let snapshot_dir = settings
        .get::<PathBuf>("snapshot_dir")
        .unwrap_or_else(|_| PathBuf::from("~/.local/share/prefault/snapshots"));

    match opts.cmd {
        Command::List { filter: _ } => {
            println!("Listing");
        }

        Command::Enable { filter: _ } => {
            println!("Enable");
        }

        Command::Disable { filter: _ } => {
            println!("Disable");
        }

        Command::Show { filter: _ } => {
            println!("Show");
        }

        Command::Snapshot { filter, pid } => match do_snapshot(filter, pid) {
            Ok(result) => {
                let result: Vec<String> =
                    result.iter().map(|p| format!("{}", p.display())).collect();
                println!("Success: {:?}", &result);
            }

            Err(e) => eprintln!("{}", e),
        },

        Command::Incore { filter, pid } => match do_mincore(filter, pid) {
            Ok(_) => println!("Success"),
            Err(e) => eprintln!("{}", e),
        },

        Command::Trace { command: _ } => {
            println!("Trace is currently not implemented");
        }

        Command::Remove { filter: _ } => {
            println!("Remove is currently not implemented");
        }

        Command::Cache { filter } => match do_cache(filter) {
            Ok(_) => println!("Success"),
            Err(e) => eprintln!("{}", e),
        },

        Command::Mlock { filter } => {
            do_mlock(filter, snapshot_dir).unwrap_or_else(|e| eprintln!("{}", e));

            println!("Going to sleep now");

            loop {
                thread::sleep(Duration::from_millis(2000));

                if !RUNNING.load(Ordering::SeqCst) {
                    break;
                } 
            }

            println!("Exiting");
        }
    }
}
