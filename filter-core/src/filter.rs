use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info};

use crate::api::MinifluxClient;
use crate::config::Config;
use crate::rules::{RuleSet, load_rule_sets_from_dir};

pub struct FilterEngine {
    client: MinifluxClient,
    rules_dir: String,
    poll_interval: Duration,
}

impl FilterEngine {
    pub fn new(config: &Config, rules_dir: String) -> Self {
        Self {
            client: MinifluxClient::new(config),
            rules_dir,
            poll_interval: Duration::from_secs(config.poll_interval),
        }
    }

    /// Start the main filtering loop
    pub async fn run(&self) -> Result<()> {
        info!(
            "Starting filtering engine with {} second intervals",
            self.poll_interval.as_secs()
        );

        // Test connection first
        self.client
            .test_connection()
            .await
            .context("Failed initial API connection test")?;

        loop {
            if let Err(e) = self.process_cycle().await {
                error!("Error during filtering cycle: {}", e);
                // Continue running even if a cycle fails
            }

            debug!("Sleeping for {} seconds", self.poll_interval.as_secs());
            time::sleep(self.poll_interval).await;
        }
    }

    /// Process a single filtering cycle
    async fn process_cycle(&self) -> Result<()> {
        debug!("Starting new filtering cycle");

        // Load rule sets
        let rule_sets =
            load_rule_sets_from_dir(&self.rules_dir).context("Failed to load rule sets")?;

        if rule_sets.is_empty() {
            debug!("No rule sets found, skipping cycle");
            return Ok(());
        }

        // Group rule sets by feed ID for efficient processing
        let rules_by_feed: HashMap<u64, &RuleSet> =
            rule_sets.iter().map(|rs| (rs.feed_id, rs)).collect();

        info!(
            "Processing {} rule sets for {} feeds",
            rule_sets.len(),
            rules_by_feed.len()
        );

        // Process feeds with specific rules first, then all unread entries for general rules
        let mut processed_feeds = std::collections::HashSet::new();
        let mut total_processed = 0;
        let mut total_filtered = 0;

        // Process feeds with specific rules
        for (&feed_id, rule_set) in &rules_by_feed {
            if !rule_set.is_enabled() {
                debug!("Skipping disabled rule set for feed {}", feed_id);
                continue;
            }

            let (processed, filtered) = self.process_feed(feed_id, rule_set).await?;
            total_processed += processed;
            total_filtered += filtered;
            processed_feeds.insert(feed_id);
        }

        info!(
            "Filtering cycle complete: processed {} entries, filtered {} entries",
            total_processed, total_filtered
        );

        Ok(())
    }

    /// Process entries for a specific feed with its rule set
    async fn process_feed(&self, feed_id: u64, rule_set: &RuleSet) -> Result<(usize, usize)> {
        debug!(
            "Processing feed {} with {} rules",
            feed_id,
            rule_set.rules.len()
        );

        // Fetch unread entries for this feed
        let entries = self
            .client
            .get_unread_entries_for_feed(feed_id)
            .await
            .with_context(|| format!("Failed to fetch entries for feed {}", feed_id))?;

        if entries.is_empty() {
            debug!("No unread entries for feed {}", feed_id);
            return Ok((0, 0));
        }

        let mut entries_to_mark = Vec::new();

        // Evaluate each entry against the rule set
        for entry in &entries {
            let matching_rules = rule_set.evaluate(entry);

            if !matching_rules.is_empty() {
                let rule_indices: Vec<String> =
                    matching_rules.iter().map(|i| (i + 1).to_string()).collect();
                info!(
                    "Entry '{}' (ID: {}) matches rules: {}",
                    entry.title,
                    entry.id,
                    rule_indices.join(", ")
                );
                entries_to_mark.push(entry.id);
            }
        }

        // Mark matching entries as read
        if !entries_to_mark.is_empty() {
            self.client
                .mark_entries_as_read(entries_to_mark.clone())
                .await
                .with_context(|| format!("Failed to mark entries as read for feed {}", feed_id))?;

            info!(
                "Marked {} entries as read for feed {}",
                entries_to_mark.len(),
                feed_id
            );
        }

        Ok((entries.len(), entries_to_mark.len()))
    }

    /// Get summary statistics for the current rule sets
    pub async fn get_stats(&self) -> Result<FilterStats> {
        let rule_sets = load_rule_sets_from_dir(&self.rules_dir)?;

        let total_rule_sets = rule_sets.len();
        let enabled_rule_sets = rule_sets.iter().filter(|rs| rs.is_enabled()).count();
        let total_rules = rule_sets.iter().map(|rs| rs.rules.len()).sum();

        // Get feed IDs that have rules
        let feeds_with_rules: Vec<u64> = rule_sets.iter().map(|rs| rs.feed_id).collect();

        Ok(FilterStats {
            total_rule_sets,
            enabled_rule_sets,
            total_rules,
            feeds_with_rules,
        })
    }
}

#[derive(Debug)]
pub struct FilterStats {
    pub total_rule_sets: usize,
    pub enabled_rule_sets: usize,
    pub total_rules: usize,
    pub feeds_with_rules: Vec<u64>,
}

impl FilterStats {
    pub fn print_summary(&self) {
        info!("Filter Engine Statistics:");
        info!("  Total rule sets: {}", self.total_rule_sets);
        info!("  Enabled rule sets: {}", self.enabled_rule_sets);
        info!("  Total rules: {}", self.total_rules);
        info!("  Feeds with rules: {:?}", self.feeds_with_rules);
    }
}

/// Create an example rule set file
pub fn create_example_rule_file<P: AsRef<Path>>(
    path: P,
    feed_id: u64,
    feed_name: &str,
) -> Result<()> {
    use crate::rules::{Action, Condition, Field, Operator, Rule, RuleSet};

    let example_rule_set = RuleSet {
        feed_id,
        feed_name: Some(feed_name.to_string()),
        enabled: Some(true),
        rules: vec![
            Rule {
                action: Action::MarkRead,
                conditions: vec![
                    Condition {
                        field: Field::Title,
                        operator: Operator::Contains,
                        value: "ad".to_string(),
                    },
                    Condition {
                        field: Field::Title,
                        operator: Operator::Contains,
                        value: "advertisement".to_string(),
                    },
                ],
            },
            Rule {
                action: Action::MarkRead,
                conditions: vec![Condition {
                    field: Field::Content,
                    operator: Operator::Contains,
                    value: "promotional".to_string(),
                }],
            },
            Rule {
                action: Action::MarkRead,
                conditions: vec![Condition {
                    field: Field::Author,
                    operator: Operator::Equals,
                    value: "spam-author".to_string(),
                }],
            },
        ],
    };

    example_rule_set.save_to_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_filter_engine_creation() {
        let config = Config {
            miniflux_url: "https://miniflux.example.com".to_string(),
            miniflux_token: "test-token".to_string(),
            poll_interval: 300,
            web_enabled: true,
            web_port: 8080,
        };

        let engine = FilterEngine::new(&config, "./rules".to_string());
        assert_eq!(engine.poll_interval, Duration::from_secs(300));
        assert_eq!(engine.rules_dir, "./rules");
    }
}
