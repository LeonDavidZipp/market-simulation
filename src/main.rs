mod cli;
mod manifest;
mod math;
mod order_book;
mod plot;
mod run;
mod simulation;

use clap::Parser;
use cli::Cli;
use manifest::Manifest;
use run::{RunConfig, run_simulation};
use simulation::SimulationConfig;
use std::sync::Arc;

use crate::manifest::ManifestError;

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

    let cfg = Arc::clone(&manifest.config);

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
        let manifest_json =
            serde_json::to_string_pretty(&manifest).expect("failed to serialize manifest");
        if let Err(e) = std::fs::write(cli.out.join("manifest.json"), manifest_json) {
            eprintln!("failed to write manifest: {e}");
            std::process::exit(1);
        }
    }

    let mut handles = Vec::with_capacity(manifest.n_runs);
    for num in 0..manifest.n_runs {
        let run_cfg = RunConfig {
            num,
            seed: manifest.seed.map(|s| s.wrapping_add(num as u32)),
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

fn load_manifest(cli: &Cli) -> Result<Manifest, ManifestError> {
    if let Some(path) = &cli.manifest_path {
        Manifest::from_file(path)
    } else {
        Ok(Manifest {
            seed: cli.seed,
            n_runs: cli.n_runs,
            config: Arc::new(SimulationConfig {
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
            }),
        })
    }
}
