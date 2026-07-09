use std::path::PathBuf;
use std::sync::Arc;

use crate::simulation::SimulationConfig;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub seed: Option<u32>,
    pub n_runs: usize,
    pub config: Arc<SimulationConfig>,
}

pub fn manifest_from_path(path: &PathBuf) -> Result<Manifest, ManifestError> {
    let file = std::fs::File::open(path)?;
    let manifest: Manifest = serde_json::from_reader(file)?;
    Ok(manifest)
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
