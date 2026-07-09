mod cli;
mod manifest;
mod math;
mod order_book;
mod plot;
mod run;
mod simulation;

use clap::Parser;
use cli::Cli;
use manifest::{Manifest, manifest_from_path};
use run::{RunConfig, run_simulation};
use simulation::SimulationConfig;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = cli.check_out_exists() {
        eprintln!("{e}");
        std::process::exit(1);
    }

    let cfg = if let Some(path) = &cli.manifest_path {
        let manifest = match manifest_from_path(path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        };
        Arc::new(manifest.config)
    } else {
        Arc::new(SimulationConfig {
            n_traders: cli.n_traders,
            trade_prob: cli.trade_prob,
            initial_open: cli.open,
            order_price_std: cli.order_price_std,
            skew: cli.skew,
            n_steps: cli.n_steps,
            n_ticks_per_candle: cli.n_ticks_per_candle,
            min_quantity: cli.min_quantity,
            max_quantity: cli.max_quantity,
            shock_prob: cli.shock_prob,
            shock_intensity: cli.shock_intensity,
            shock_intensity_std: cli.shock_intensity_std,
            spike_ratio: cli.spike_ratio,
        })
    };

    let data_dir = cli.out.join("data");
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        eprintln!("failed to create output directory: {e}");
        std::process::exit(1);
    }

    let chart_dir = cli.out.join("charts");
    if cli.with_charts
        && let Err(e) = std::fs::create_dir_all(&chart_dir)
    {
        eprintln!("failed to create chart output directory: {e}");
        std::process::exit(1);
    }

    if cli.with_manifest {
        let manifest = Manifest {
            seed: cli.seed,
            n_runs: cli.n_runs,
            config: (*cfg).clone(),
        };
        let manifest_json =
            serde_json::to_string_pretty(&manifest).expect("failed to serialize manifest");
        if let Err(e) = std::fs::write(cli.out.join("manifest.json"), manifest_json) {
            eprintln!("failed to write manifest: {e}");
            std::process::exit(1);
        }
    }

    let mut handles = Vec::with_capacity(cli.n_runs);
    for num in 0..cli.n_runs {
        let run_cfg = RunConfig {
            num,
            seed: cli.seed.map(|s| s.wrapping_add(num as u32)),
            simulation_cfg: Arc::clone(&cfg),
            out: data_dir.join(format!("run{num}.parquet")),
            chart_out: cli
                .with_charts
                .then(|| chart_dir.join(format!("run{num}.svg"))),
        };
        handles.push(tokio::spawn(run_simulation(run_cfg)));
    }

    for handle in handles {
        handle.await.expect("run task panicked");
    }
}
