//! feheim — a Homebrew package manager core, rebuilt in Rust.
//!
//! Consumes the real formulae.brew.sh formula index and installs real
//! bottles from ghcr.io into a private prefix.

mod api;
mod commands;
mod config;
mod error;
mod formula;
mod install;
mod platform;
mod relocate;
mod state;
mod ui;

use clap::{Parser, Subcommand};
use config::Config;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "feheim",
    version,
    about = "A Homebrew package manager core, rebuilt in Rust"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Refresh the cached formula index.
    Update,
    /// Search formulae by name or description.
    Search { query: String },
    /// Show details for a formula.
    Info { name: String },
    /// List installed formulae.
    List,
    /// Install a formula and its dependencies.
    Install { name: String },
    /// Uninstall a formula.
    Uninstall { name: String },
    /// Remove orphaned dependencies no longer required by any requested formula.
    Cleanup,
    /// Diagnose the installation: dead links, orphans, index, and keg status.
    Doctor,
}

fn main() -> ExitCode {
    // Behave like a normal Unix tool when our output is piped into `head` etc.:
    // restore default SIGPIPE so we exit quietly instead of panicking on a
    // broken pipe (Rust ignores SIGPIPE by default).
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let cli = Cli::parse();
    let cfg = Config::load();

    let result = match cli.command {
        Command::Update => commands::update(&cfg),
        Command::Search { query } => commands::search(&cfg, &query),
        Command::Info { name } => commands::info(&cfg, &name),
        Command::List => commands::list(&cfg),
        Command::Install { name } => commands::install(&cfg, &name),
        Command::Uninstall { name } => commands::uninstall(&cfg, &name),
        Command::Cleanup => commands::cleanup(&cfg),
        Command::Doctor => commands::doctor(&cfg),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{} {e}", ui::error_label());
            ExitCode::FAILURE
        }
    }
}
