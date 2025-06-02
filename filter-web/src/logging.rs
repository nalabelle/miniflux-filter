use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::{Layer, Registry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub target: String,
    pub feed_id: Option<u64>,
    pub entry_id: Option<u64>,
    pub entry_title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WebLogCollector {
    logs: Arc<Mutex<VecDeque<LogEntry>>>,
    max_logs: usize,
}

impl WebLogCollector {
    pub fn new(max_logs: usize) -> Self {
        Self {
            logs: Arc::new(Mutex::new(VecDeque::with_capacity(max_logs))),
            max_logs,
        }
    }

    pub fn add_log(&self, entry: LogEntry) {
        let mut logs = self.logs.lock().unwrap();

        // Remove oldest entries if we're at capacity
        while logs.len() >= self.max_logs {
            logs.pop_front();
        }

        logs.push_back(entry);
    }

    pub fn get_logs(&self) -> Vec<LogEntry> {
        let logs = self.logs.lock().unwrap();
        logs.iter().cloned().collect()
    }

    pub fn get_recent_logs(&self, limit: usize) -> Vec<LogEntry> {
        let logs = self.logs.lock().unwrap();
        logs.iter().rev().take(limit).cloned().collect()
    }

    pub fn get_logs_for_feed(&self, feed_id: u64, limit: Option<usize>) -> Vec<LogEntry> {
        let logs = self.logs.lock().unwrap();
        let filtered: Vec<LogEntry> = logs
            .iter()
            .filter(|entry| entry.feed_id == Some(feed_id))
            .cloned()
            .collect();

        if let Some(limit) = limit {
            filtered.into_iter().rev().take(limit).collect()
        } else {
            filtered.into_iter().rev().collect()
        }
    }

    pub fn clear_logs(&self) {
        let mut logs = self.logs.lock().unwrap();
        logs.clear();
    }
}

pub struct WebLogLayer {
    collector: WebLogCollector,
}

impl WebLogLayer {
    pub fn new(collector: WebLogCollector) -> Self {
        Self { collector }
    }
}

impl<S> Layer<S> for WebLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();

        // Only capture logs from our filter modules
        let target = metadata.target();
        if !target.starts_with("filter_core")
            && !target.starts_with("filter_web")
            && !target.starts_with("miniflux_filter")
        {
            return;
        }

        let mut visitor = LogVisitor::new();
        event.record(&mut visitor);

        let entry = LogEntry {
            timestamp: Utc::now(),
            level: metadata.level().to_string(),
            message: visitor.message,
            target: target.to_string(),
            feed_id: visitor.feed_id,
            entry_id: visitor.entry_id,
            entry_title: visitor.entry_title,
        };

        self.collector.add_log(entry);
    }
}

struct LogVisitor {
    message: String,
    feed_id: Option<u64>,
    entry_id: Option<u64>,
    entry_title: Option<String>,
}

impl LogVisitor {
    fn new() -> Self {
        Self {
            message: String::new(),
            feed_id: None,
            entry_id: None,
            entry_title: None,
        }
    }
}

impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "message" => {
                self.message = format!("{:?}", value);
                // Remove quotes from debug formatting
                if self.message.starts_with('"') && self.message.ends_with('"') {
                    self.message = self.message[1..self.message.len() - 1].to_string();
                }
            }
            "feed_id" => {
                if let Ok(id) = format!("{:?}", value).parse::<u64>() {
                    self.feed_id = Some(id);
                }
            }
            "entry_id" => {
                if let Ok(id) = format!("{:?}", value).parse::<u64>() {
                    self.entry_id = Some(id);
                }
            }
            "entry_title" => {
                let title = format!("{:?}", value);
                if title.starts_with('"') && title.ends_with('"') {
                    self.entry_title = Some(title[1..title.len() - 1].to_string());
                } else {
                    self.entry_title = Some(title);
                }
            }
            _ => {}
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "message" => self.message = value.to_string(),
            "entry_title" => self.entry_title = Some(value.to_string()),
            _ => {}
        }
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        match field.name() {
            "feed_id" => self.feed_id = Some(value),
            "entry_id" => self.entry_id = Some(value),
            _ => {}
        }
    }
}

pub fn setup_web_logging(
    max_logs: usize,
    log_level: &str,
) -> (impl Subscriber + Send + Sync, WebLogCollector) {
    let level = match log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    let collector = WebLogCollector::new(max_logs);
    let web_layer = WebLogLayer::new(collector.clone());

    let subscriber = Registry::default()
        .with(
            tracing_subscriber::fmt::layer()
                .with_filter(tracing_subscriber::filter::LevelFilter::from_level(level)),
        )
        .with(web_layer.with_filter(tracing_subscriber::filter::LevelFilter::from_level(level)));

    (subscriber, collector)
}
