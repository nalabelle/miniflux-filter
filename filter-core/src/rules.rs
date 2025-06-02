use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::api::Entry;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuleSet {
    pub feed_id: u64,
    pub feed_name: Option<String>,
    pub enabled: Option<bool>,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    pub action: Action,
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    MarkRead,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Condition {
    pub field: Field,
    pub operator: Operator,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Field {
    Title,
    Content,
    Author,
    Url,
    Tag,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Operator {
    Contains,
    NotContains,
    Equals,
    NotEquals,
    StartsWith,
    EndsWith,
    Matches, // For regex
}

impl RuleSet {
    /// Load a rule set from a TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        debug!("Loading rule set from {}", path.display());

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read rule file: {}", path.display()))?;

        let rule_set: RuleSet = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML rule file: {}", path.display()))?;

        rule_set.validate()?;

        info!(
            "Loaded rule set for feed {} with {} rules",
            rule_set.feed_id,
            rule_set.rules.len()
        );

        Ok(rule_set)
    }

    /// Save a rule set to a TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        debug!("Saving rule set to {}", path.display());

        self.validate()?;

        let content =
            toml::to_string_pretty(self).context("Failed to serialize rule set to TOML")?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        fs::write(path, content)
            .with_context(|| format!("Failed to write rule file: {}", path.display()))?;

        info!(
            "Saved rule set for feed {} to {}",
            self.feed_id,
            path.display()
        );

        Ok(())
    }

    /// Validate the rule set
    pub fn validate(&self) -> Result<()> {
        if self.rules.is_empty() {
            warn!("Rule set for feed {} has no rules", self.feed_id);
        }

        for (i, rule) in self.rules.iter().enumerate() {
            if rule.conditions.is_empty() {
                anyhow::bail!("Rule {} has no conditions", i + 1);
            }

            for (j, condition) in rule.conditions.iter().enumerate() {
                if condition.value.trim().is_empty() {
                    anyhow::bail!("Rule {} condition {} has an empty value", i + 1, j + 1);
                }

                // Validate regex patterns if using Matches operator
                if let Operator::Matches = condition.operator {
                    regex::Regex::new(&condition.value).with_context(|| {
                        format!(
                            "Invalid regex pattern in rule {} condition {}: '{}'",
                            i + 1,
                            j + 1,
                            condition.value
                        )
                    })?;
                }
            }
        }

        Ok(())
    }

    /// Check if the rule set is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }

    /// Evaluate all rules against an entry and return matching rule indices
    pub fn evaluate(&self, entry: &Entry) -> Vec<usize> {
        if !self.is_enabled() {
            return Vec::new();
        }

        let mut matching_rules = Vec::new();

        for (i, rule) in self.rules.iter().enumerate() {
            if self.evaluate_rule(rule, entry) {
                debug!("Entry {} matches rule {}", entry.id, i + 1);
                matching_rules.push(i);
            }
        }

        matching_rules
    }

    /// Evaluate a single rule against an entry
    fn evaluate_rule(&self, rule: &Rule, entry: &Entry) -> bool {
        // All conditions must be true for the rule to match
        rule.conditions
            .iter()
            .all(|condition| self.evaluate_condition(condition, entry))
    }

    /// Evaluate a single condition against an entry
    fn evaluate_condition(&self, condition: &Condition, entry: &Entry) -> bool {
        let field_value = match condition.field {
            Field::Title => &entry.title,
            Field::Content => &entry.content,
            Field::Author => &entry.author,
            Field::Url => &entry.url,
            Field::Tag => {
                // For tags, we check if any tag matches the condition
                let tags_joined = entry.tags.join(" ");
                return match condition.operator {
                    Operator::Contains => tags_joined
                        .to_lowercase()
                        .contains(&condition.value.to_lowercase()),
                    Operator::NotContains => !tags_joined
                        .to_lowercase()
                        .contains(&condition.value.to_lowercase()),
                    Operator::Equals => entry
                        .tags
                        .iter()
                        .any(|tag| tag.eq_ignore_ascii_case(&condition.value)),
                    Operator::NotEquals => !entry
                        .tags
                        .iter()
                        .any(|tag| tag.eq_ignore_ascii_case(&condition.value)),
                    Operator::StartsWith => entry.tags.iter().any(|tag| {
                        tag.to_lowercase()
                            .starts_with(&condition.value.to_lowercase())
                    }),
                    Operator::EndsWith => entry.tags.iter().any(|tag| {
                        tag.to_lowercase()
                            .ends_with(&condition.value.to_lowercase())
                    }),
                    Operator::Matches => match regex::Regex::new(&condition.value) {
                        Ok(re) => entry.tags.iter().any(|tag| re.is_match(tag)),
                        Err(_) => {
                            warn!("Invalid regex pattern '{}' in condition", condition.value);
                            false
                        }
                    },
                };
            }
        };

        match condition.operator {
            Operator::Contains => field_value
                .to_lowercase()
                .contains(&condition.value.to_lowercase()),
            Operator::NotContains => !field_value
                .to_lowercase()
                .contains(&condition.value.to_lowercase()),
            Operator::Equals => field_value.eq_ignore_ascii_case(&condition.value),
            Operator::NotEquals => !field_value.eq_ignore_ascii_case(&condition.value),
            Operator::StartsWith => field_value
                .to_lowercase()
                .starts_with(&condition.value.to_lowercase()),
            Operator::EndsWith => field_value
                .to_lowercase()
                .ends_with(&condition.value.to_lowercase()),
            Operator::Matches => match regex::Regex::new(&condition.value) {
                Ok(re) => re.is_match(field_value),
                Err(_) => {
                    warn!("Invalid regex pattern '{}' in condition", condition.value);
                    false
                }
            },
        }
    }
}

/// Load all rule sets from a directory
pub fn load_rule_sets_from_dir<P: AsRef<Path>>(dir_path: P) -> Result<Vec<RuleSet>> {
    let dir_path = dir_path.as_ref();

    if !dir_path.exists() {
        info!(
            "Rules directory {} does not exist, creating it",
            dir_path.display()
        );
        fs::create_dir_all(dir_path)
            .with_context(|| format!("Failed to create rules directory: {}", dir_path.display()))?;
        return Ok(Vec::new());
    }

    let mut rule_sets = Vec::new();

    for entry in fs::read_dir(dir_path)
        .with_context(|| format!("Failed to read rules directory: {}", dir_path.display()))?
    {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            match RuleSet::load_from_file(&path) {
                Ok(rule_set) => rule_sets.push(rule_set),
                Err(e) => {
                    warn!("Failed to load rule file {}: {}", path.display(), e);
                }
            }
        }
    }

    info!(
        "Loaded {} rule sets from {}",
        rule_sets.len(),
        dir_path.display()
    );
    Ok(rule_sets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Entry, Feed};

    #[test]
    fn test_rule_evaluation() {
        let rule_set = RuleSet {
            feed_id: 123,
            feed_name: Some("Test Feed".to_string()),
            enabled: Some(true),
            rules: vec![Rule {
                action: Action::MarkRead,
                conditions: vec![Condition {
                    field: Field::Title,
                    operator: Operator::Contains,
                    value: "advertisement".to_string(),
                }],
            }],
        };

        let entry = Entry {
            id: 1,
            title: "This is an Advertisement".to_string(),
            url: "https://example.com".to_string(),
            content: "Some content".to_string(),
            author: "Author".to_string(),
            status: "unread".to_string(),
            feed: Feed {
                id: 123,
                title: "Test Feed".to_string(),
                site_url: "https://example.com".to_string(),
                feed_url: "https://example.com/feed".to_string(),
            },
            published_at: "2024-01-01T00:00:00Z".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            tags: vec![],
        };

        let matches = rule_set.evaluate(&entry);
        assert_eq!(matches, vec![0]); // First rule (index 0)
    }

    #[test]
    fn test_disabled_rule_set() {
        let rule_set = RuleSet {
            feed_id: 123,
            feed_name: Some("Test Feed".to_string()),
            enabled: Some(false),
            rules: vec![Rule {
                action: Action::MarkRead,
                conditions: vec![Condition {
                    field: Field::Title,
                    operator: Operator::Contains,
                    value: "test".to_string(),
                }],
            }],
        };

        let entry = Entry {
            id: 1,
            title: "This is a test".to_string(),
            url: "https://example.com".to_string(),
            content: "Some content".to_string(),
            author: "Author".to_string(),
            status: "unread".to_string(),
            feed: Feed {
                id: 123,
                title: "Test Feed".to_string(),
                site_url: "https://example.com".to_string(),
                feed_url: "https://example.com/feed".to_string(),
            },
            published_at: "2024-01-01T00:00:00Z".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            tags: vec![],
        };

        let matches = rule_set.evaluate(&entry);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_tag_evaluation() {
        let rule_set = RuleSet {
            feed_id: 123,
            feed_name: Some("Test Feed".to_string()),
            enabled: Some(true),
            rules: vec![Rule {
                action: Action::MarkRead,
                conditions: vec![Condition {
                    field: Field::Tag,
                    operator: Operator::Matches,
                    value: "(?i)sports".to_string(),
                }],
            }],
        };

        let entry = Entry {
            id: 1,
            title: "Test Article".to_string(),
            url: "https://example.com".to_string(),
            content: "Some content".to_string(),
            author: "Author".to_string(),
            status: "unread".to_string(),
            feed: Feed {
                id: 123,
                title: "Test Feed".to_string(),
                site_url: "https://example.com".to_string(),
                feed_url: "https://example.com/feed".to_string(),
            },
            published_at: "2024-01-01T00:00:00Z".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            tags: vec!["News".to_string(), "Sports".to_string()],
        };

        let matches = rule_set.evaluate(&entry);
        assert_eq!(matches, vec![0]); // First rule (index 0)
    }
}
