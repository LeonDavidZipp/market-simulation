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
use std::path::PathBuf;
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

    let (data_dir, chart_dir) = create_fs_artifacts(&cli, &manifest).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    let mut handles = Vec::with_capacity(manifest.n_runs);
    for num in 0..manifest.n_runs {
        let run_cfg = RunConfig {
            num,
            seed: manifest.seed.map(|s| s.wrapping_add(num as u32)),
            simulation_cfg: Arc::clone(&cfg),
            out: data_dir.join(format!("run{num}.parquet")),
            chart_out: chart_dir
                .as_ref()
                .map(|c_dir| c_dir.join(format!("run{num}.svg"))),
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

/// Creates the `data`/`charts` output directories and, if enabled, writes
/// `manifest.json`. Returns the `data`/`charts` directory paths.
///
/// # Errors
///
/// Returns [`FsArtifactsError`] if a directory can't be created, the
/// manifest can't be serialized, or `manifest.json` can't be written.
fn create_fs_artifacts(
    cli: &Cli,
    manifest: &Manifest,
) -> Result<(PathBuf, Option<PathBuf>), FsArtifactsError> {
    let data_dir = cli.out.join("data");
    std::fs::create_dir_all(&data_dir)?;

    let chart_dir = if cli.with_charts {
        Some(cli.out.join("charts"))
    } else {
        None
    };
    if let Some(c_dir) = &chart_dir {
        std::fs::create_dir_all(c_dir)?;
    };

    if cli.with_manifest {
        let manifest_json = serde_json::to_string_pretty(manifest)?;
        std::fs::write(cli.out.join("manifest.json"), manifest_json)?;
    }

    Ok((data_dir, chart_dir))
}

#[derive(Debug)]
enum FsArtifactsError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for FsArtifactsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsArtifactsError::Io(e) => write!(f, "io error: {e}"),
            FsArtifactsError::Json(e) => write!(f, "failed to serialize manifest: {e}"),
        }
    }
}

impl std::error::Error for FsArtifactsError {}

impl From<std::io::Error> for FsArtifactsError {
    fn from(e: std::io::Error) -> Self {
        FsArtifactsError::Io(e)
    }
}

impl From<serde_json::Error> for FsArtifactsError {
    fn from(e: serde_json::Error) -> Self {
        FsArtifactsError::Json(e)
    }
}
