use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "UltraWM",
    version = ultrawm_core::version(),
    about = "UltraWM - A next-generation, cross-platform tiling window manager",
)]
pub struct Args {
    #[arg(
        short = 'c',
        long = "config",
        value_name = "FILE",
        help = "Specify custom configuration file path"
    )]
    pub config_path: Option<PathBuf>,

    #[arg(
        long = "validate",
        help = "Validate configuration and exit without starting"
    )]
    pub validate: bool,

    #[arg(
        long = "defaults",
        help = "Use default configuration and ignore config files"
    )]
    pub use_defaults: bool,

    #[arg(long = "no-persistence", help = "Disable saving and loading of layout")]
    pub no_persistence: bool,

    #[arg(
        long = "reset-layout",
        help = "Deletes your current layout in case it has issues"
    )]
    pub reset_layout: bool,

    #[arg(long = "quiet", help = "Enable quiet logging mode (info)")]
    pub quiet: bool,

    #[arg(long = "verbose", help = "Enable verbose logging mode (trace)")]
    pub verbose: bool,

    #[arg(long = "console", help = "Run in a console window (Windows only)")]
    pub console: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            config_path: None,
            validate: false,
            use_defaults: false,
            no_persistence: false,
            reset_layout: false,
            quiet: false,
            verbose: false,
            console: false,
        }
    }
}

pub fn parse_args() -> Args {
    Args::parse()
}
