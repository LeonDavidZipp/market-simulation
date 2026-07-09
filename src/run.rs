use crate::plot;
use crate::simulation::{Simulation, SimulationConfig, SimulationError};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

pub struct RunConfig {
    pub num: usize,
    pub seed: Option<u32>,
    pub simulation_cfg: Arc<SimulationConfig>,
    pub out: PathBuf,
    pub chart_out: Option<PathBuf>,
}

pub async fn run_simulation(cfg: RunConfig) {
    let m_cfg = cfg.simulation_cfg;
    let seed = cfg.seed;
    let sim_start = Instant::now();
    let simulation = tokio::task::spawn_blocking(move || {
        let mut simulation = Simulation::with_config(m_cfg, seed);
        simulation.run()?;
        Ok::<_, SimulationError>(simulation)
    })
    .await
    .expect("simulation task panicked");

    let simulation = match simulation {
        Ok(s) => s,
        Err(e) => {
            eprintln!("run {} simulation failed: {e}", cfg.num);
            std::process::exit(1);
        }
    };
    println!(
        "run {} simulation took {:.3?}",
        cfg.num,
        sim_start.elapsed()
    );

    let simulation = Arc::new(simulation);
    let save_start = Instant::now();

    let write_simulation = Arc::clone(&simulation);
    let out_path = cfg.out.clone();
    let write_handle =
        tokio::task::spawn_blocking(move || write_output(&write_simulation, &out_path));

    let chart_handle = cfg.chart_out.clone().map(|chart_path| {
        let chart_simulation = Arc::clone(&simulation);
        tokio::task::spawn_blocking(move || write_chart(&chart_simulation, &chart_path))
    });

    if let Err(e) = write_handle.await.expect("data-saving task panicked") {
        eprintln!("run {} error saving data: {e}", cfg.num);
        std::process::exit(1);
    }

    if let Some(handle) = chart_handle
        && let Err(e) = handle.await.expect("chart-saving task panicked")
    {
        eprintln!("run {} error saving chart: {e}", cfg.num);
        std::process::exit(1);
    }

    println!(
        " run {} output saving took {:.3?}",
        cfg.num,
        save_start.elapsed()
    );
}

fn write_output(simulation: &Simulation, out: &PathBuf) -> Result<(), String> {
    let extension = out.extension().and_then(|e| e.to_str());
    let file = || File::create(out).map_err(|e| e.to_string());

    match extension {
        Some("csv") => simulation
            .history_to_csv(file()?)
            .map_err(|e| e.to_string()),
        Some("parquet") => simulation
            .history_to_parquet(file()?)
            .map_err(|e| e.to_string()),
        Some(other) => Err(format!("unsupported output file extension: .{other}")),
        None => Err("output file has no extension; cannot infer format".to_string()),
    }
}

fn write_chart(simulation: &Simulation, out: &Path) -> Result<(), String> {
    let df = simulation.history_to_df().map_err(|e| e.to_string())?;
    let path = out
        .to_str()
        .ok_or_else(|| "chart output path is not valid UTF-8".to_string())?;
    plot::plot_candles(&df, path).map_err(|e| e.to_string())
}
