use anyhow::Result;
use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, Json, Response},
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

use filter_core::api::MinifluxClient;
use filter_core::rules::{RuleSet, load_rule_sets_from_dir};

#[derive(Clone)]
pub struct WebState {
    pub rules_dir: String,
    pub miniflux_client: MinifluxClient,
    pub log_collector: Option<crate::logging::WebLogCollector>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct FeedInfo {
    pub id: u64,
    pub title: String,
    pub site_url: String,
    pub feed_url: String,
    pub has_rules: bool,
}

#[derive(Deserialize)]
pub struct CreateRuleSetRequest {
    pub feed_id: u64,
}

pub async fn start_web_server(
    rules_dir: String,
    miniflux_client: MinifluxClient,
    port: u16,
    log_collector: Option<crate::logging::WebLogCollector>,
) -> Result<()> {
    let state = WebState {
        rules_dir,
        miniflux_client,
        log_collector,
    };

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/style.css", get(serve_css))
        .route("/app.js", get(serve_app_js))
        .route("/edit.js", get(serve_edit_js))
        .route("/lib/api.js", get(serve_api_js))
        .route("/edit.html", get(serve_edit_html))
        .route("/api/rules", get(list_rule_sets))
        .route("/api/rules", post(create_rule_set))
        .route("/api/rules/{feed_id}", get(get_rule_set))
        .route("/api/rules/{feed_id}", put(update_rule_set))
        .route("/api/rules/{feed_id}", delete(delete_rule_set))
        .route("/api/feeds", get(list_feeds))
        .route("/api/feeds/{feed_id}", get(get_feed))
        .route("/api/stats", get(get_stats))
        .route("/api/execute/{feed_id}", post(execute_filter))
        .route("/api/logs", get(get_logs))
        .route("/api/logs/{feed_id}", get(get_logs_for_feed))
        .route("/api/logs", delete(clear_logs))
        .layer(ServiceBuilder::new().layer(CorsLayer::permissive()))
        .with_state(Arc::new(state));

    let addr = format!("0.0.0.0:{}", port);
    info!("Starting web UI server on http://{}", addr);

    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", addr, e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Web server error: {}", e))?;

    Ok(())
}

// Embedded static files
const INDEX_HTML: &str = include_str!("../static/index.html");
const EDIT_HTML: &str = include_str!("../static/edit.html");
const STYLE_CSS: &str = include_str!("../static/style.css");
const APP_JS: &str = include_str!("../static/app.js");
const EDIT_JS: &str = include_str!("../static/edit.js");
const API_JS: &str = include_str!("../static/lib/api.js");

async fn serve_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn serve_edit_html() -> Html<&'static str> {
    Html(EDIT_HTML)
}

async fn serve_css() -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "text/css")
        .body(STYLE_CSS.into())
        .unwrap()
}

async fn serve_app_js() -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/javascript")
        .body(APP_JS.into())
        .unwrap()
}

async fn serve_edit_js() -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/javascript")
        .body(EDIT_JS.into())
        .unwrap()
}

async fn serve_api_js() -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/javascript")
        .body(API_JS.into())
        .unwrap()
}

async fn list_rule_sets(State(state): State<Arc<WebState>>) -> Json<ApiResponse<Vec<RuleSet>>> {
    match load_rule_sets_from_dir(&state.rules_dir) {
        Ok(rule_sets) => Json(ApiResponse {
            success: true,
            data: Some(rule_sets),
            error: None,
        }),
        Err(e) => {
            error!("Failed to load rule sets: {}", e);
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            })
        }
    }
}

async fn get_rule_set(
    Path(feed_id): Path<u64>,
    State(state): State<Arc<WebState>>,
) -> Result<Json<ApiResponse<RuleSet>>, StatusCode> {
    let rule_sets = match load_rule_sets_from_dir(&state.rules_dir) {
        Ok(sets) => sets,
        Err(e) => {
            error!("Failed to load rule sets: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if let Some(rule_set) = rule_sets.into_iter().find(|rs| rs.feed_id == feed_id) {
        Ok(Json(ApiResponse {
            success: true,
            data: Some(rule_set),
            error: None,
        }))
    } else {
        Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Rule set for feed {} not found", feed_id)),
        }))
    }
}

async fn create_rule_set(
    State(state): State<Arc<WebState>>,
    Json(request): Json<CreateRuleSetRequest>,
) -> Json<ApiResponse<String>> {
    let rule_set = RuleSet {
        feed_id: request.feed_id,
        enabled: true,
        rules: Vec::new(),
    };

    let filename = format!("{}/feed_{}.toml", state.rules_dir, request.feed_id);

    match rule_set.save_to_file(&filename) {
        Ok(_) => {
            info!("Created new rule set for feed {}", request.feed_id);
            Json(ApiResponse {
                success: true,
                data: Some(format!("Rule set created for feed {}", request.feed_id)),
                error: None,
            })
        }
        Err(e) => {
            error!("Failed to create rule set: {}", e);
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            })
        }
    }
}

async fn update_rule_set(
    Path(feed_id): Path<u64>,
    State(state): State<Arc<WebState>>,
    Json(rule_set): Json<RuleSet>,
) -> Json<ApiResponse<String>> {
    if rule_set.feed_id != feed_id {
        return Json(ApiResponse {
            success: false,
            data: None,
            error: Some("Feed ID mismatch".to_string()),
        });
    }

    let filename = format!("{}/feed_{}.toml", state.rules_dir, feed_id);

    match rule_set.save_to_file(&filename) {
        Ok(_) => {
            info!("Updated rule set for feed {}", feed_id);
            Json(ApiResponse {
                success: true,
                data: Some(format!("Rule set updated for feed {}", feed_id)),
                error: None,
            })
        }
        Err(e) => {
            error!("Failed to update rule set: {}", e);
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            })
        }
    }
}

async fn delete_rule_set(
    Path(feed_id): Path<u64>,
    State(state): State<Arc<WebState>>,
) -> Json<ApiResponse<String>> {
    // Find the actual rule file for this feed ID by scanning the directory
    let rules_dir = std::path::Path::new(&state.rules_dir);

    if !rules_dir.exists() {
        return Json(ApiResponse {
            success: false,
            data: None,
            error: Some("Rules directory does not exist".to_string()),
        });
    }

    // Look for any TOML file that contains this feed_id
    let dir_entries = match std::fs::read_dir(rules_dir) {
        Ok(entries) => entries,
        Err(e) => {
            error!("Failed to read rules directory: {}", e);
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to read rules directory: {}", e)),
            });
        }
    };

    for entry in dir_entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            // Try to parse this TOML file to see if it matches our feed_id
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(rule_set) = toml::from_str::<RuleSet>(&content) {
                    if rule_set.feed_id == feed_id {
                        // This is the file we want to delete
                        match std::fs::remove_file(&path) {
                            Ok(_) => {
                                info!("Deleted rule set for feed {} from {:?}", feed_id, path);
                                return Json(ApiResponse {
                                    success: true,
                                    data: Some(format!("Rule set deleted for feed {}", feed_id)),
                                    error: None,
                                });
                            }
                            Err(e) => {
                                error!("Failed to delete rule file {:?}: {}", path, e);
                                return Json(ApiResponse {
                                    success: false,
                                    data: None,
                                    error: Some(format!("Failed to delete rule file: {}", e)),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Json(ApiResponse {
        success: false,
        data: None,
        error: Some(format!("Rule set for feed {} not found", feed_id)),
    })
}

async fn list_feeds(State(state): State<Arc<WebState>>) -> Json<ApiResponse<Vec<FeedInfo>>> {
    // Get feeds from Miniflux API
    let feeds_result = state.miniflux_client.get_feeds().await;

    let feeds = match feeds_result {
        Ok(feeds) => feeds,
        Err(e) => {
            error!("Failed to fetch feeds from Miniflux: {}", e);
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to fetch feeds: {}", e)),
            });
        }
    };

    // Get existing rule sets to determine which feeds have rules
    let rule_sets = load_rule_sets_from_dir(&state.rules_dir).unwrap_or_default();
    let feeds_with_rules: std::collections::HashSet<u64> =
        rule_sets.into_iter().map(|rs| rs.feed_id).collect();

    let feed_info: Vec<FeedInfo> = feeds
        .into_iter()
        .map(|feed| FeedInfo {
            id: feed.id,
            title: feed.title,
            site_url: feed.site_url,
            feed_url: feed.feed_url,
            has_rules: feeds_with_rules.contains(&feed.id),
        })
        .collect();

    Json(ApiResponse {
        success: true,
        data: Some(feed_info),
        error: None,
    })
}

async fn get_feed(
    Path(feed_id): Path<u64>,
    State(state): State<Arc<WebState>>,
) -> Json<ApiResponse<FeedInfo>> {
    // Get feeds from Miniflux API
    let feeds_result = state.miniflux_client.get_feeds().await;

    let feeds = match feeds_result {
        Ok(feeds) => feeds,
        Err(e) => {
            error!("Failed to fetch feeds from Miniflux: {}", e);
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to fetch feeds: {}", e)),
            });
        }
    };

    // Find the specific feed
    let feed = feeds.into_iter().find(|f| f.id == feed_id);

    match feed {
        Some(feed) => {
            // Check if this feed has rules
            let rule_sets = load_rule_sets_from_dir(&state.rules_dir).unwrap_or_default();
            let has_rules = rule_sets.iter().any(|rs| rs.feed_id == feed_id);

            let feed_info = FeedInfo {
                id: feed.id,
                title: feed.title,
                site_url: feed.site_url,
                feed_url: feed.feed_url,
                has_rules,
            };

            Json(ApiResponse {
                success: true,
                data: Some(feed_info),
                error: None,
            })
        }
        None => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Feed with ID {} not found", feed_id)),
        }),
    }
}

async fn get_stats(State(state): State<Arc<WebState>>) -> Json<ApiResponse<serde_json::Value>> {
    let rule_sets = load_rule_sets_from_dir(&state.rules_dir).unwrap_or_default();

    let total_rule_sets = rule_sets.len();
    let enabled_rule_sets = rule_sets.iter().filter(|rs| rs.is_enabled()).count();
    let total_rules = rule_sets.iter().map(|rs| rs.rules.len()).sum::<usize>();

    let stats = serde_json::json!({
        "total_rule_sets": total_rule_sets,
        "enabled_rule_sets": enabled_rule_sets,
        "total_rules": total_rules,
        "feeds_with_rules": rule_sets.iter().map(|rs| rs.feed_id).collect::<Vec<_>>()
    });

    Json(ApiResponse {
        success: true,
        data: Some(stats),
        error: None,
    })
}

#[derive(Serialize)]
pub struct ExecuteResult {
    pub processed: usize,
    pub filtered: usize,
    pub message: String,
}

async fn execute_filter(
    Path(feed_id): Path<u64>,
    State(state): State<Arc<WebState>>,
) -> Json<ApiResponse<ExecuteResult>> {
    // Load the rule set for this feed
    let rule_sets = match load_rule_sets_from_dir(&state.rules_dir) {
        Ok(sets) => sets,
        Err(e) => {
            error!("Failed to load rule sets: {}", e);
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to load rule sets: {}", e)),
            });
        }
    };

    // Find the rule set for this feed
    let rule_set = match rule_sets.into_iter().find(|rs| rs.feed_id == feed_id) {
        Some(rs) => rs,
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("No rule set found for feed {}", feed_id)),
            });
        }
    };

    if !rule_set.is_enabled() {
        return Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Rule set for feed {} is disabled", feed_id)),
        });
    }

    // Fetch unread entries for this feed
    let entries = match state
        .miniflux_client
        .get_unread_entries_for_feed(feed_id)
        .await
    {
        Ok(entries) => entries,
        Err(e) => {
            error!("Failed to fetch entries for feed {}: {}", feed_id, e);
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to fetch entries: {}", e)),
            });
        }
    };

    if entries.is_empty() {
        return Json(ApiResponse {
            success: true,
            data: Some(ExecuteResult {
                processed: 0,
                filtered: 0,
                message: "No unread entries found for this feed".to_string(),
            }),
            error: None,
        });
    }

    let mut entries_to_mark = Vec::new();

    // Evaluate each entry against the rule set
    for entry in &entries {
        let matching_rules = rule_set.evaluate(entry);
        if !matching_rules.is_empty() {
            entries_to_mark.push(entry.id);
        }
    }

    // Mark matching entries as read
    if !entries_to_mark.is_empty() {
        if let Err(e) = state
            .miniflux_client
            .mark_entries_as_read(entries_to_mark.clone())
            .await
        {
            error!("Failed to mark entries as read for feed {}: {}", feed_id, e);
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to mark entries as read: {}", e)),
            });
        }
    }

    let message = if entries_to_mark.is_empty() {
        format!(
            "Processed {} entries, no entries matched the rules",
            entries.len()
        )
    } else {
        format!(
            "Processed {} entries, marked {} as read",
            entries.len(),
            entries_to_mark.len()
        )
    };

    Json(ApiResponse {
        success: true,
        data: Some(ExecuteResult {
            processed: entries.len(),
            filtered: entries_to_mark.len(),
            message,
        }),
        error: None,
    })
}

async fn get_logs(
    State(state): State<Arc<WebState>>,
) -> Json<ApiResponse<Vec<crate::logging::LogEntry>>> {
    match &state.log_collector {
        Some(collector) => {
            let logs = collector.get_recent_logs(50); // Get last 50 logs
            Json(ApiResponse {
                success: true,
                data: Some(logs),
                error: None,
            })
        }
        None => Json(ApiResponse {
            success: false,
            data: None,
            error: Some("Logging not enabled".to_string()),
        }),
    }
}

async fn get_logs_for_feed(
    Path(feed_id): Path<u64>,
    State(state): State<Arc<WebState>>,
) -> Json<ApiResponse<Vec<crate::logging::LogEntry>>> {
    match &state.log_collector {
        Some(collector) => {
            let logs = collector.get_logs_for_feed(feed_id, Some(50)); // Get last 50 logs for this feed
            Json(ApiResponse {
                success: true,
                data: Some(logs),
                error: None,
            })
        }
        None => Json(ApiResponse {
            success: false,
            data: None,
            error: Some("Logging not enabled".to_string()),
        }),
    }
}

async fn clear_logs(State(state): State<Arc<WebState>>) -> Json<ApiResponse<String>> {
    match &state.log_collector {
        Some(collector) => {
            collector.clear_logs();
            Json(ApiResponse {
                success: true,
                data: Some("Logs cleared successfully".to_string()),
                error: None,
            })
        }
        None => Json(ApiResponse {
            success: false,
            data: None,
            error: Some("Logging not enabled".to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
    };
    use filter_core::rules::{Action, Condition, Field, Operator, Rule};
    use std::sync::Arc;
    use tempfile::TempDir;
    use tower::ServiceExt;

    // Helper function to create a test web app
    fn create_test_app(rules_dir: String) -> Router {
        use filter_core::config::Config;

        let config = Config {
            miniflux_url: "http://test.example.com".to_string(),
            miniflux_token: "test-api-key".to_string(),
            poll_interval: 300,
            web_enabled: true,
            web_port: 8080,
        };

        let miniflux_client = MinifluxClient::new(&config);

        let state = WebState {
            rules_dir,
            miniflux_client,
            log_collector: None,
        };

        Router::new()
            .route("/api/rules/{feed_id}", put(update_rule_set))
            .with_state(Arc::new(state))
    }

    #[tokio::test]
    async fn test_submit_rule_success() {
        // Create a temporary directory for rules
        let temp_dir = TempDir::new().unwrap();
        let rules_dir = temp_dir.path().to_string_lossy().to_string();

        // Create the test app
        let app = create_test_app(rules_dir);

        // Create a rule set with a valid rule
        let rule_set = RuleSet {
            feed_id: 123,
            enabled: true,
            rules: vec![Rule {
                action: Action::MarkRead,
                conditions: vec![Condition {
                    field: Field::Title,
                    operator: Operator::Contains,
                    value: "test".to_string(),
                }],
            }],
        };

        // Create the request
        let request = Request::builder()
            .method("PUT")
            .uri("/api/rules/123")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&rule_set).unwrap()))
            .unwrap();

        // Send the request
        let response = app.oneshot(request).await.unwrap();

        // Verify the response
        assert_eq!(response.status(), StatusCode::OK);

        // Parse the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let api_response: ApiResponse<String> = serde_json::from_slice(&body).unwrap();

        // Verify that the request succeeded
        assert!(api_response.success);
        assert!(api_response.error.is_none());
    }
}
