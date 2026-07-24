use clap::Parser;

use shmod::cli::Cli;

fn main() {
    if let Err(e) = shmod::run(Cli::parse()) {
        eprintln!("\x1b[0;31m[ERROR]\x1b[0m {e:#}");
        std::process::exit(1);
    }
}
