use clap::Parser;

/// Evaluate, validate and export cuecue configurations.
#[derive(Parser)]
#[command(version)]
struct Cli {}

fn main() {
    Cli::parse();
}
