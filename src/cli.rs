use std::path::PathBuf;

use clap::Parser;
use pimalaya_tui::cli::arg::path_parser;

#[derive(Parser, Debug)]
#[command(name = "himalaya", author, version, about)]
#[command(propagate_version = true, infer_subcommands = true)]
pub struct Cli {
    /// Override the default configuration file path.
    ///
    /// The given paths are shell-expanded then canonicalized (if
    /// applicable). If the first path does not point to a valid file,
    /// the wizard will propose to assist you in the creation of the
    /// configuration file. Other paths are merged with the first one,
    /// which allows you to separate your public config from your
    /// private(s) one(s).
    #[arg(short, long = "config", global = true, env = "HIMALAYA_CONFIG")]
    #[arg(value_name = "PATH", value_parser = path_parser)]
    pub config_paths: Vec<PathBuf>,

    /// Enable logs with spantrace.
    ///
    /// This is the same as running the command with `RUST_LOG=debug`
    /// environment variable.
    #[arg(long, global = true, conflicts_with = "trace")]
    pub debug: bool,

    /// Enable verbose logs with backtrace.
    ///
    /// This is the same as running the command with `RUST_LOG=trace`
    /// and `RUST_BACKTRACE=1` environment variables.
    #[arg(long, global = true, conflicts_with = "debug")]
    pub trace: bool,
}
