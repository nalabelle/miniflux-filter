use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct MinifluxClient {
    client: Client,
    base_url: String,
    token: String,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    pub id: u64,
    pub title: String,
    pub url: String,
    pub content: String,
    pub author: String,
    pub status: String,
    pub feed: Feed,
    pub published_at: String,
    pub created_at: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Feed {
    pub id: u64,
    pub title: String,
    pub site_url: String,
    pub feed_url: String,
}

#[derive(Debug, Deserialize)]
pub struct EntriesResponse {
    pub total: u64,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Serialize)]
pub struct MarkEntriesRequest {
    pub entry_ids: Vec<u64>,
    pub status: String,
}

impl MinifluxClient {
    pub fn new(config: &Config) -> Self {
        let client = Client::new();

        Self {
            client,
            base_url: config.miniflux_url.clone(),
            token: config.miniflux_token.clone(),
        }
    }

    /// Test the API connection and authentication
    pub async fn test_connection(&self) -> Result<()> {
        debug!("Testing Miniflux API connection");

        let url = format!("{}/v1/me", self.base_url);
        let response = self
            .client
            .get(&url)
            .header("X-Auth-Token", &self.token)
            .send()
            .await
            .context("Failed to connect to Miniflux API")?;

        if response.status().is_success() {
            info!("Miniflux API connection successful");
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Miniflux API connection failed: {} - {}", status, text);
        }
    }

    /// Fetch all unread entries
    pub async fn get_unread_entries(&self) -> Result<Vec<Entry>> {
        debug!("Fetching unread entries from Miniflux");

        let url = format!("{}/v1/entries?status=unread&limit=1000", self.base_url);
        let response = self
            .client
            .get(&url)
            .header("X-Auth-Token", &self.token)
            .send()
            .await
            .context("Failed to fetch unread entries")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to fetch unread entries: {} - {}", status, text);
        }

        let entries_response: EntriesResponse = response
            .json()
            .await
            .context("Failed to parse entries response")?;

        info!("Fetched {} unread entries", entries_response.entries.len());
        Ok(entries_response.entries)
    }

    /// Fetch unread entries for a specific feed
    pub async fn get_unread_entries_for_feed(&self, feed_id: u64) -> Result<Vec<Entry>> {
        debug!("Fetching unread entries for feed {}", feed_id);

        let url = format!(
            "{}/v1/feeds/{}/entries?status=unread&limit=1000",
            self.base_url, feed_id
        );
        let response = self
            .client
            .get(&url)
            .header("X-Auth-Token", &self.token)
            .send()
            .await
            .context("Failed to fetch unread entries for feed")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to fetch unread entries for feed {}: {} - {}",
                feed_id,
                status,
                text
            );
        }

        let entries_response: EntriesResponse = response
            .json()
            .await
            .context("Failed to parse entries response")?;

        debug!(
            "Fetched {} unread entries for feed {}",
            entries_response.entries.len(),
            feed_id
        );
        Ok(entries_response.entries)
    }

    /// Fetch all feeds
    pub async fn get_feeds(&self) -> Result<Vec<Feed>> {
        debug!("Fetching feeds from Miniflux");

        let url = format!("{}/v1/feeds", self.base_url);
        let response = self
            .client
            .get(&url)
            .header("X-Auth-Token", &self.token)
            .send()
            .await
            .context("Failed to fetch feeds")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to fetch feeds: {} - {}", status, text);
        }

        let feeds: Vec<Feed> = response
            .json()
            .await
            .context("Failed to parse feeds response")?;

        info!("Fetched {} feeds", feeds.len());
        Ok(feeds)
    }

    /// Mark entries as read
    pub async fn mark_entries_as_read(&self, entry_ids: Vec<u64>) -> Result<()> {
        if entry_ids.is_empty() {
            return Ok(());
        }

        debug!("Marking {} entries as read", entry_ids.len());

        let url = format!("{}/v1/entries", self.base_url);
        let request = MarkEntriesRequest {
            entry_ids: entry_ids.clone(),
            status: "read".to_string(),
        };

        let response = self
            .client
            .put(&url)
            .header("X-Auth-Token", &self.token)
            .json(&request)
            .send()
            .await
            .context("Failed to mark entries as read")?;

        if response.status().is_success() {
            info!("Successfully marked {} entries as read", entry_ids.len());
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to mark entries as read: {} - {}", status, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = Config {
            miniflux_url: "https://miniflux.example.com".to_string(),
            miniflux_token: "test-token".to_string(),
            poll_interval: 300,
            web_enabled: true,
            web_port: 8080,
        };

        let client = MinifluxClient::new(&config);
        assert_eq!(client.base_url, "https://miniflux.example.com");
        assert_eq!(client.token, "test-token");
    }
}
