use crate::simulation::SimulationConfig;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub seed: Option<u32>,
    pub n_runs: usize,
    pub config: SimulationConfig,
}
