use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "shmod",
    about = "Context-selective bash configuration loader",
    version
)]
pub struct Cli {
    /// Override the module tree root (else SHMOD_ROOT, XDG config dir, or ~/.local/shmod).
    #[arg(long, global = true)]
    pub root: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Emit shell integration: SHMOD_ROOT, the shmod() shim, and startup sourcing.
    Init {
        /// Target shell (only bash supported today).
        #[arg(value_enum, default_value_t = Shell::Bash)]
        shell: Shell,
    },
    /// Emit source lines for a profile's modules; --save persists it as default.
    Use {
        profile: String,
        #[arg(long)]
        save: bool,
    },
    /// Emit source lines for the given files/dirs (":"-separated allowed).
    Source {
        /// Module paths relative to root; multiple or ":"-separated.
        paths: Vec<String>,
        /// Source regardless of extension or ".off" marker.
        #[arg(long)]
        force: bool,
    },
    /// Re-source the full active environment (startup list + active profile) into this shell.
    Reload,
    /// Clear the active profile for this shell; --save clears the persisted default.
    Reset {
        #[arg(long)]
        save: bool,
    },
    /// List profiles defined in shmod.yaml.
    Profiles,
    /// Show the persisted default profile.
    Active,
    /// List modules discovered under the tree.
    List {
        #[arg(long, value_enum, default_value_t = ListMode::All)]
        mode: ListMode,
    },
    /// Report declared @dep commands missing from PATH.
    Check {
        /// Module paths relative to root; multiple or ":"-separated.
        paths: Vec<String>,
    },
}

#[derive(Copy, Clone, ValueEnum)]
pub enum Shell {
    Bash,
}

#[derive(Copy, Clone, ValueEnum, PartialEq, Eq)]
pub enum ListMode {
    All,
    Enabled,
    Disabled,
}

/// Split each arg on ':' to support the legacy `a:b:c` multi-path syntax.
pub fn split_paths(args: &[String]) -> Vec<String> {
    args.iter()
        .flat_map(|a| a.split(':'))
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}
