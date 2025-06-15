use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "UltraWM",
    version = ultrawm_core::version(),
    about = "UltraWM - A next-generation, cross-platform tiling window manager",
)]
pub struct Args {
    /// Specify custom configuration file path
    #[arg(
        short = 'c',
        long = "config",
        value_name = "FILE",
        help = "Specify custom configuration file path"
    )]
    pub config_path: Option<PathBuf>,

    /// Validate configuration and exit without starting
    #[arg(
        long = "dry-run",
        help = "Validate configuration and exit without starting"
    )]
    pub dry_run: bool,

    /// Use default configuration and ignore config files
    #[arg(
        long = "defaults",
        help = "Use default configuration and ignore config files"
    )]
    pub use_defaults: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            config_path: None,
            dry_run: false,
            use_defaults: false,
        }
    }
}

pub fn parse_args() -> Args {
    Args::parse()
}
