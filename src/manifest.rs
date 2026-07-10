use std::path::PathBuf;
use std::sync::Arc;

use crate::{cli::Cli, simulation::SimulationConfig};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub seed: Option<u32>,
    pub n_runs: u32,
    pub config: Arc<SimulationConfig>,
}

impl Manifest {
    /// Reads and parses a [`Manifest`] from the JSON file at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::Io`] if the file cannot be opened, or
    /// [`ManifestError::Json`] if its contents are not a valid `Manifest`.
    pub fn from_file(path: &PathBuf) -> Result<Manifest, ManifestError> {
        let file = std::fs::File::open(path)?;
        let manifest: Manifest = serde_json::from_reader(file)?;
        Ok(manifest)
    }

    pub fn from_cli(cli: &Cli) -> Manifest {
        Manifest {
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
        }
    }
}

/// Builds the run [`Manifest`], either by loading it from [`Cli::manifest_path`]
/// if given, or by constructing it from the individual CLI arguments.
///
/// # Errors
///
/// Returns [`ManifestError`] if a manifest path is given but the file can't
/// be read or parsed.
pub fn load_manifest(cli: &Cli) -> Result<Manifest, ManifestError> {
    if let Some(path) = &cli.manifest_path {
        Manifest::from_file(path)
    } else {
        Ok(Manifest::from_cli(cli))
    }
}

#[derive(Debug)]
pub enum ManifestError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::Io(e) => write!(f, "io error: {e}"),
            ManifestError::Json(e) => write!(f, "invalid manifest json: {e}"),
        }
    }
}

impl std::error::Error for ManifestError {}

impl From<std::io::Error> for ManifestError {
    fn from(e: std::io::Error) -> Self {
        ManifestError::Io(e)
    }
}

impl From<serde_json::Error> for ManifestError {
    fn from(e: serde_json::Error) -> Self {
        ManifestError::Json(e)
    }
}
