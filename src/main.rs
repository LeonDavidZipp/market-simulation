mod cli;
mod manifest;
mod math;
mod order_book;
mod plot;
mod run;
mod simulation;

use clap::Parser;
use cli::Cli;
use manifest::{Manifest, load_manifest};
use run::run_multiple_simulations;
use std::path::PathBuf;

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

    let (data_dir, chart_dir) = create_fs_artifacts(&cli, &manifest).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    run_multiple_simulations(&manifest, &data_dir, chart_dir).await;
}

/// Creates the `data` output directory, and the `charts` output directory if
/// [`Cli::with_charts`] is set. If [`Cli::with_manifest`] is set, writes
/// `manifest.json`. If [`Cli::allow_overwrite`] is set and [`Cli::out`]
/// already exists, it is removed first so it can be recreated from scratch.
/// Returns the `data` directory path, and the `charts` directory path if it
/// was created.
///
/// # Errors
///
/// Returns [`FsArtifactsError`] if the existing output directory can't be
/// removed, a directory can't be created, the manifest can't be serialized,
/// or `manifest.json` can't be written.
fn create_fs_artifacts(
    cli: &Cli,
    manifest: &Manifest,
) -> Result<(PathBuf, Option<PathBuf>), FsArtifactsError> {
    if cli.allow_overwrite && cli.out.try_exists()? {
        std::fs::remove_dir_all(&cli.out)?;
    }
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
