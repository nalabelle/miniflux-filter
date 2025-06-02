mod cli;

use anyhow::Result;
use filter_core::api::MinifluxClient;
use filter_core::config::Config;
use filter_core::filter::FilterEngine;
use filter_web::{setup_web_logging, start_web_server};
use std::env;
use tokio::try_join;
use tracing::{error, info};

use crate::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse_args();

    // Initialize web logging with tracing
    let (subscriber, log_collector) = setup_web_logging(50, &cli.log_level);
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    info!("Starting miniflux-filter v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration from environment variables
    let config = match Config::from_env() {
        Ok(config) => {
            info!("Configuration loaded successfully");
            info!("Miniflux URL: {}", config.miniflux_url);
            info!("Poll interval: {} seconds", config.poll_interval);
            info!(
                "Web UI: {}",
                if config.web_enabled {
                    format!("enabled on port {}", config.web_port)
                } else {
                    "disabled".to_string()
                }
            );
            config
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            error!("Required environment variables:");
            error!("  MINIFLUX_URL - URL of your Miniflux instance");
            error!("  MINIFLUX_API_TOKEN - Your Miniflux API token");
            error!("Optional environment variables:");
            error!("  MINIFLUX_FILTER_POLL_INTERVAL - Polling interval in seconds (default: 300)");
            error!("  MINIFLUX_FILTER_WEB_ENABLED - Enable web UI (default: true)");
            error!("  MINIFLUX_FILTER_WEB_PORT - Web UI port (default: 8080)");
            return Err(e);
        }
    };

    // Get rules directory from environment or use default
    let rules_dir = env::var("MINIFLUX_FILTER_RULES_DIR").unwrap_or_else(|_| "./rules".to_string());

    info!("Using rules directory: {}", rules_dir);

    // Create filtering engine
    let filter_engine = FilterEngine::new(&config, rules_dir.clone());

    // Show initial statistics
    match filter_engine.get_stats().await {
        Ok(stats) => {
            stats.print_summary();

            if stats.total_rule_sets == 0 {
                info!("No rule sets found in {}", rules_dir);
                info!("Create TOML rule files in the rules directory to start filtering");
                if config.web_enabled {
                    info!(
                        "Or use the web interface at http://localhost:{} to configure rules",
                        config.web_port
                    );
                }
                info!("Application will continue running and check for new rules every cycle");
            }
        }
        Err(e) => {
            error!("Failed to get initial statistics: {}", e);
        }
    }

    // Start services concurrently
    if config.web_enabled {
        info!("Starting web server and filtering engine...");

        // Create Miniflux client for web server
        let web_client = MinifluxClient::new(&config);

        // Run both web server and filtering engine concurrently
        try_join!(
            start_web_server(rules_dir, web_client, config.web_port, Some(log_collector)),
            filter_engine.run()
        )?;
    } else {
        info!("Starting filtering engine (web UI disabled)...");
        filter_engine.run().await?;
    }

    Ok(())
}
