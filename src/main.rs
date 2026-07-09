mod cli;
mod fs_artifacts;
mod manifest;
mod math;
mod order_book;
mod plot;
mod run;
mod simulation;

use clap::Parser;
use cli::Cli;
use fs_artifacts::create_fs_artifacts;
use manifest::load_manifest;
use run::run_multiple_simulations;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = cli.check_out_exists() {
        eprintln!("{e}");
        std::process::exit(1);
    }

    let manifest = load_manifest(&cli).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    let (data_dir, chart_dir) = create_fs_artifacts(&cli, &manifest).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    run_multiple_simulations(&manifest, &data_dir, chart_dir).await;
}
