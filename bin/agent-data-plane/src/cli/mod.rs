use std::path::PathBuf;

use argh::FromArgs;

mod config;
pub use self::config::handle_config_command;
use self::config::ConfigCommand;

mod debug;
pub use self::debug::handle_debug_command;
use self::debug::DebugCommand;

mod dogstatsd;
pub use self::dogstatsd::handle_dogstatsd_command;
use self::dogstatsd::DogstatsdCommand;

mod run;
use self::run::RunCommand;

mod runtime;
pub use self::runtime::handle_run_command;

mod runtime_setup;
pub use self::runtime_setup::RuntimeShell;

mod utils;

mod version;
pub use self::version::handle_version_command;
use self::version::VersionCommand;

#[derive(FromArgs)]
#[argh(
    description = "Data plane for the Datadog Agent.",
    help_triggers("-h", "--help", "help")
)]
pub struct Cli {
    /// path to the configuration file
    #[argh(option, short = 'c', long = "config")]
    pub config_file: Option<PathBuf>,

    /// subcommand to run
    #[argh(subcommand)]
    pub action: Action,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum Action {
    Run(RunCommand),
    Debug(DebugCommand),
    Config(ConfigCommand),
    Dogstatsd(DogstatsdCommand),
    Version(VersionCommand),
}
