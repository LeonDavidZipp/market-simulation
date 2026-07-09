use clap::Parser;
use std::path::{Path, PathBuf};

#[derive(Parser)]
pub struct Cli {
    #[arg(long = "from-manifest")]
    pub manifest_path: Option<PathBuf>,

    #[arg(
        short = 's',
        long = "seed",
        help = "Base RNG seed; with --n-runs > 1, run N uses seed + N so each run is distinct but reproducible"
    )]
    pub seed: Option<u32>,

    #[arg(short = 'o', long = "out", default_value_os_t = default_out_dir())]
    pub out: PathBuf,

    #[arg(long = "with-charts", default_value_t = false)]
    pub with_charts: bool,

    #[arg(long = "with-manifest", default_value_t = true)]
    pub with_manifest: bool,

    #[arg(long = "allow-overwrite", default_value_t = false)]
    pub allow_overwrite: bool,

    #[arg(short = 'n', long = "n-steps", default_value_t = 100)]
    pub n_steps: usize,

    #[arg(long = "n-runs", visible_alias = "nr", default_value_t = 1)]
    pub n_runs: usize,

    #[arg(long = "n-traders", visible_alias = "nt", default_value_t = 1000)]
    pub n_traders: usize,

    #[arg(long = "trade-prob", visible_alias = "tp", default_value_t = 0.0005)]
    pub trade_prob: f32,

    #[arg(long = "ticks-per-candle", visible_alias = "tpc", default_value_t = 10)]
    pub n_ticks_per_candle: usize,

    #[arg(long = "open", visible_alias = "op", default_value_t = 100.0)]
    pub open: f32,

    #[arg(
        long = "order-price-std",
        visible_alias = "ops",
        default_value_t = 0.01
    )]
    pub order_price_std: f32,

    #[arg(long = "skew", visible_alias = "sk", default_value_t = 0.0)]
    pub skew: f32,

    #[arg(long = "min-quantity", visible_alias = "mnq", default_value_t = 1.0)]
    pub min_quantity: f32,

    #[arg(long = "max-quantity", visible_alias = "mxq", default_value_t = 10.0)]
    pub max_quantity: f32,

    #[arg(long = "shock-prob", visible_alias = "shp", default_value_t = 0.0)]
    pub shock_prob: f32,

    #[arg(long = "shock-intensity", visible_alias = "shi", default_value_t = 0.3)]
    pub shock_intensity: f32,

    #[arg(
        long = "shock-intensity-std",
        visible_alias = "shs",
        default_value_t = 0.2
    )]
    pub shock_intensity_std: f32,

    #[arg(long = "spike-ratio", visible_alias = "sr", default_value_t = 0.5)]
    pub spike_ratio: f32,
}

impl Cli {
    /// Checks whether [`Cli::out`] already exists, refusing to proceed unless
    /// [`Cli::allow_overwrite`] is set.
    ///
    /// # Errors
    ///
    /// Returns [`CliError::Io`] if the filesystem check itself fails, or
    /// [`CliError::FileExists`] if the path exists and overwriting isn't allowed.
    pub fn check_out_exists(&self) -> Result<(), CliError> {
        let exists = Path::new(&self.out).try_exists()?;
        if exists && !self.allow_overwrite {
            return Err(CliError::FileExists(self.out.clone()));
        }
        Ok(())
    }
}

/// Returns the default `--out` directory: `<current working directory>/simulation`.
fn default_out_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("simulation")
}

#[derive(Debug)]
pub enum CliError {
    Io(std::io::Error),
    FileExists(PathBuf),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Io(e) => write!(f, "io error: {e}"),
            CliError::FileExists(path) => write!(
                f,
                "output path already exists: {} (pass --allow-overwrite to overwrite it)",
                path.display()
            ),
        }
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e)
    }
}
