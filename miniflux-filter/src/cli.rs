//! Command-line interface for miniflux-filter

use clap::Parser;

#[derive(Parser)]
#[command(name = "miniflux-filter")]
#[command(about = "Extended filtering capabilities for Miniflux RSS reader")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    /// Log level
    #[arg(long, env = "MINIFLUX_FILTER_LOG_LEVEL", default_value = "info")]
    pub log_level: String,
}

impl Cli {
    /// Parse command line arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }
}