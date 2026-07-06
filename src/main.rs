mod market;
mod math;
mod order_book;

use clap::Parser;
use market::{Market, MarketConfig};
use std::fs::File;
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    #[arg(short = 'n', long = "n-runs", default_value_t = 100)]
    n_runs: usize,

    #[arg(short = 'o', long = "out")]
    out: PathBuf,

    #[arg(long = "n-traders", visible_alias = "nt", default_value_t = 1000)]
    n_traders: usize,

    #[arg(long = "trade-prob", visible_alias = "tp", default_value_t = 0.0005)]
    trade_prob: f32,

    #[arg(long = "ticks-per-candle", visible_alias = "tpc", default_value_t = 10)]
    n_ticks_per_candle: usize,

    #[arg(long = "open", visible_alias = "op", default_value_t = 100.0)]
    open: f32,

    #[arg(long = "open-std", visible_alias = "os", default_value_t = 0.01)]
    open_std: f32,

    #[arg(long = "skew", visible_alias = "sk", default_value_t = 0.0)]
    skew: f32,

    #[arg(long = "min-quantity", visible_alias = "mnq", default_value_t = 1.0)]
    min_quantity: f32,

    #[arg(long = "max-quantity", visible_alias = "mxq", default_value_t = 10.0)]
    max_quantity: f32,

    #[arg(
        long = "buyer-ratio-std",
        visible_alias = "brs",
        default_value_t = 0.05
    )]
    buyer_ratio_std: f32,
}

fn main() {
    let cli = Cli::parse();

    let cfg = MarketConfig {
        n_traders: cli.n_traders,
        trade_prob: cli.trade_prob,
        initial_open: cli.open,
        open_std: cli.open_std,
        skew: cli.skew,
        n_runs: cli.n_runs,
        n_ticks_per_candle: cli.n_ticks_per_candle,
        min_quantity: cli.min_quantity,
        max_quantity: cli.max_quantity,
        buyer_ratio_std: cli.buyer_ratio_std,
    };

    let mut market = Market::with_config(cfg);
    if let Err(e) = market.run() {
        eprintln!("simulation failed: {e:?}");
        std::process::exit(1);
    }

    if let Err(e) = write_output(&market, &cli.out) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn write_output(market: &Market, out: &PathBuf) -> Result<(), String> {
    let extension = out.extension().and_then(|e| e.to_str());
    let file = || File::create(out).map_err(|e| e.to_string());

    match extension {
        Some("csv") => market.history_to_csv(file()?).map_err(|e| e.to_string()),
        Some("parquet") => market
            .history_to_parquet(file()?)
            .map_err(|e| e.to_string()),
        Some(other) => Err(format!("unsupported output file extension: .{other}")),
        None => Err("output file has no extension; cannot infer format".to_string()),
    }
}
