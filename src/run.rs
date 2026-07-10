use crate::manifest::Manifest;
use crate::plot;
use crate::simulation::{Simulation, SimulationConfig, SimulationError};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

/// Spawns [`manifest.n_runs`](Manifest::n_runs) simulation runs concurrently,
/// each writing its parquet output under `data_dir` and, if `chart_dir` is
/// set, its chart under `chart_dir`. Waits for every run to finish before
/// returning.
///
/// # Panics
///
/// Panics if any spawned run task itself panics.
pub async fn run_multiple_simulations(
    manifest: &Manifest,
    data_dir: &Path,
    chart_dir: Option<PathBuf>,
) {
    let cfg = Arc::clone(&manifest.config);
    let mut handles = Vec::with_capacity(manifest.n_runs as usize);
    for num in 0..manifest.n_runs {
        let run_cfg = RunConfig {
            num,
            seed: manifest.seed.map(|s| s.wrapping_add(num)),
            simulation_cfg: Arc::clone(&cfg),
            out: data_dir.join(format!("run_{num}.parquet")),
            chart_out: chart_dir
                .as_ref()
                .map(|c_dir| c_dir.join(format!("run_{num}.svg"))),
        };
        handles.push(tokio::spawn(run_simulation(run_cfg)));
    }

    for handle in handles {
        handle.await.expect("run task panicked");
    }
}

pub struct RunConfig {
    pub num: u32,
    pub seed: Option<u32>,
    pub simulation_cfg: Arc<SimulationConfig>,
    pub out: PathBuf,
    pub chart_out: Option<PathBuf>,
}

/// Runs a single simulation to completion, then concurrently writes its data
/// output and, if configured, its candlestick chart.
///
/// The simulation itself and both write steps run via
/// [`tokio::task::spawn_blocking`] so this CPU-bound work never blocks an
/// async worker thread. On any failure this prints an error and exits the
/// process.
async fn run_simulation(cfg: RunConfig) {
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

/// Writes `simulation`'s history to `out`, inferring CSV or Parquet format
/// from the file extension.
///
/// # Errors
///
/// Returns an error if `out` has no extension, an unsupported extension, or
/// if writing the file fails.
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

/// Renders `simulation`'s candlestick chart to `out` as an SVG file.
///
/// # Errors
///
/// Returns an error if the history can't be converted to a `DataFrame`, `out`
/// isn't valid UTF-8, or rendering the chart fails.
fn write_chart(simulation: &Simulation, out: &Path) -> Result<(), String> {
    let df = simulation.history_to_df().map_err(|e| e.to_string())?;
    let path = out
        .to_str()
        .ok_or_else(|| "chart output path is not valid UTF-8".to_string())?;
    plot::plot_candles(&df, path).map_err(|e| e.to_string())
}
