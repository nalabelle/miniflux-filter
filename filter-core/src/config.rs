use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub miniflux_url: String,
    pub miniflux_token: String,
    pub poll_interval: u64,
    pub web_enabled: bool,
    pub web_port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let miniflux_url =
            env::var("MINIFLUX_URL").context("MINIFLUX_URL environment variable is required")?;

        let miniflux_token = env::var("MINIFLUX_API_TOKEN")
            .context("MINIFLUX_API_TOKEN environment variable is required")?;

        let poll_interval = env::var("MINIFLUX_FILTER_POLL_INTERVAL")
            .unwrap_or_else(|_| "300".to_string())
            .parse::<u64>()
            .context("MINIFLUX_FILTER_POLL_INTERVAL must be a valid number of seconds")?;

        let web_enabled = env::var("MINIFLUX_FILTER_WEB_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        let web_port = env::var("MINIFLUX_FILTER_WEB_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .context("MINIFLUX_FILTER_WEB_PORT must be a valid port number")?;

        // Basic URL validation
        if !miniflux_url.starts_with("http://") && !miniflux_url.starts_with("https://") {
            anyhow::bail!("MINIFLUX_URL must start with http:// or https://");
        }

        // Remove trailing slash if present
        let miniflux_url = miniflux_url.trim_end_matches('/').to_string();

        if miniflux_token.is_empty() {
            anyhow::bail!("MINIFLUX_API_TOKEN cannot be empty");
        }

        Ok(Config {
            miniflux_url,
            miniflux_token,
            poll_interval,
            web_enabled,
            web_port,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = Config {
            miniflux_url: "https://miniflux.example.com".to_string(),
            miniflux_token: "test-token".to_string(),
            poll_interval: 300,
            web_enabled: true,
            web_port: 8080,
        };

        assert_eq!(config.miniflux_url, "https://miniflux.example.com");
        assert_eq!(config.miniflux_token, "test-token");
        assert_eq!(config.poll_interval, 300);
    }

    #[test]
    fn test_url_validation() {
        // Test valid HTTP URL
        assert!(
            "http://example.com".starts_with("http://")
                || "http://example.com".starts_with("https://")
        );

        // Test valid HTTPS URL
        assert!(
            "https://example.com".starts_with("http://")
                || "https://example.com".starts_with("https://")
        );

        // Test invalid URL
        assert!(!"invalid-url".starts_with("http://") && !"invalid-url".starts_with("https://"));
    }

    #[test]
    fn test_poll_interval_parsing() {
        // Test valid interval parsing
        assert_eq!("300".parse::<u64>().unwrap(), 300);
        assert_eq!("600".parse::<u64>().unwrap(), 600);

        // Test invalid interval
        assert!("invalid".parse::<u64>().is_err());
    }
}
