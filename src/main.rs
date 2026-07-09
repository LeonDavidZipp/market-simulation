mod manifest;
mod math;
mod order_book;
mod plot;
mod simulation;

use clap::Parser;
use manifest::Manifest;
use simulation::{Simulation, SimulationConfig, SimulationError};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let cfg = Arc::new(SimulationConfig {
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
    });

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

#[derive(Parser)]
struct Cli {
    #[arg(
        short = 's',
        long = "seed",
        help = "Base RNG seed; with --n-runs > 1, run N uses seed + N so each run is distinct but reproducible"
    )]
    seed: Option<u32>,

    #[arg(short = 'o', long = "out", default_value_os_t = default_out_dir())]
    out: PathBuf,

    #[arg(long = "with-charts", default_value_t = false)]
    with_charts: bool,

    #[arg(long = "with-manifest", default_value_t = true)]
    with_manifest: bool,

    #[arg(short = 'n', long = "n-steps", default_value_t = 100)]
    n_steps: usize,

    #[arg(long = "n-runs", visible_alias = "nr", default_value_t = 1)]
    n_runs: usize,

    #[arg(long = "n-traders", visible_alias = "nt", default_value_t = 1000)]
    n_traders: usize,

    #[arg(long = "trade-prob", visible_alias = "tp", default_value_t = 0.0005)]
    trade_prob: f32,

    #[arg(long = "ticks-per-candle", visible_alias = "tpc", default_value_t = 10)]
    n_ticks_per_candle: usize,

    #[arg(long = "open", visible_alias = "op", default_value_t = 100.0)]
    open: f32,

    #[arg(
        long = "order-price-std",
        visible_alias = "ops",
        default_value_t = 0.01
    )]
    order_price_std: f32,

    #[arg(long = "skew", visible_alias = "sk", default_value_t = 0.0)]
    skew: f32,

    #[arg(long = "min-quantity", visible_alias = "mnq", default_value_t = 1.0)]
    min_quantity: f32,

    #[arg(long = "max-quantity", visible_alias = "mxq", default_value_t = 10.0)]
    max_quantity: f32,

    #[arg(long = "shock-prob", visible_alias = "shp", default_value_t = 0.0)]
    shock_prob: f32,

    #[arg(long = "shock-intensity", visible_alias = "shi", default_value_t = 0.3)]
    shock_intensity: f32,

    #[arg(
        long = "shock-intensity-std",
        visible_alias = "shs",
        default_value_t = 0.2
    )]
    shock_intensity_std: f32,

    #[arg(long = "spike-ratio", visible_alias = "sr", default_value_t = 0.5)]
    spike_ratio: f32,
}

struct RunConfig {
    num: usize,
    seed: Option<u32>,
    simulation_cfg: Arc<SimulationConfig>,
    out: PathBuf,
    chart_out: Option<PathBuf>,
}

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

fn default_out_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("simulation")
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
