use std::path::PathBuf;

use clap::Parser;
use log::{error, info};

#[derive(Parser)]
#[command(name = "ublx")]
struct Args {
    /// Directory to index (default: current directory)
    #[arg(default_value = ".")]
    dir: PathBuf,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let args = Args::parse();
    let dir = validate_dir(&args.dir);

    info!("indexing directory: {}", dir.display());
}

fn validate_dir(path: &std::path::Path) -> PathBuf {
    if path.exists() && !path.is_dir() {
        error!("ublx: '{}' is not a directory", path.display());
        std::process::exit(1);
    }
    if !path.exists() {
        error!("ublx: '{}' no such file or directory", path.display());
        std::process::exit(1);
    }
    path.canonicalize().unwrap_or_else(|e| {
        error!("ublx: cannot canonicalize '{}': {}", path.display(), e);
        std::process::exit(1);
    })
}
