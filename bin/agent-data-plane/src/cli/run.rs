//! The `run` subcommand definition.
//!
//! The runtime orchestration lives in [`crate::cli::runtime`]; this module only declares the
//! command-line surface so the command definition stays free of configuration-loading concerns.

use std::path::PathBuf;

use argh::FromArgs;

/// Runs the data plane.
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "run")]
pub struct RunCommand {
    /// path to the PID file
    #[argh(option, short = 'p', long = "pidfile")]
    pub pid_file: Option<PathBuf>,
}
