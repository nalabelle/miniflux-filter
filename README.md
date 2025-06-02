# miniflux-filter

A Rust application to extend Miniflux RSS reader's filtering capabilities with advanced rule-based
automation and a web-based management interface.

## Overview

This application polls your Miniflux instance for unread entries and applies custom filtering rules
to automatically mark matching entries as read. Rules can be managed through either a web interface
or TOML configuration files, with one file per feed for precise and flexible content filtering.

## Key Features

- **Web Management Interface**: Easy-to-use web UI for creating and managing filter rules
- **Feed-Specific Rules**: Create custom filtering rules for individual feeds
- **Multiple Filter Conditions**: Filter by title, content, author, or URL
- **Flexible Operators**: Contains, equals, starts with, ends with, and regex matching
- **Real-time Logging**: Web dashboard showing filtering activity and statistics
- **Environment Configuration**: 12-factor app principles with environment variables
- **Polling-Based**: Configurable intervals for checking new entries
- **Stateless Design**: No local database required
- **Docker Support**: Pre-built containers ready to deploy

## Quick Start

```bash
docker run -d \
  -p 8080:8080 \
  -e MINIFLUX_URL=https://your-miniflux.example.com \
  -e MINIFLUX_API_TOKEN=your-api-token \
  -v ./rules:/app/rules \
  ghcr.io/nalabelle/miniflux-filter:latest
```

Then visit `http://localhost:8080` to access the web interface.

## Configuration

Set the following environment variables:

### Required

- `MINIFLUX_URL` - URL of your Miniflux instance (e.g., `https://miniflux.example.com`)
- `MINIFLUX_API_TOKEN` - Your Miniflux API token

### Optional

- `MINIFLUX_FILTER_WEB_ENABLED` - Enable web UI (default: `true`)
- `MINIFLUX_FILTER_WEB_PORT` - Web UI port (default: `8080`)
- `MINIFLUX_FILTER_POLL_INTERVAL` - Polling interval in seconds (default: `300`)
- `MINIFLUX_FILTER_RULES_DIR` - Rules directory path (default: `/app/rules`)

## Usage

### Web Interface (Recommended)

1. Start the container using the Docker command above
2. Open your browser to `http://localhost:8080`
3. Use the web interface to:
   - View all your feeds with filtering statistics
   - Create and manage filter rules with a user-friendly form
   - Monitor real-time filtering activity
   - Toggle rules on/off without restarting

## Rule Configuration

### Web Interface (Recommended)

The web interface provides an intuitive form for creating rules:

1. Navigate to the web interface at `http://localhost:8080`
2. Click "Edit Rules" next to any feed
3. Add rules using the form interface
4. Rules are automatically saved and applied

### Manual TOML Configuration

You can also create TOML files in the `rules/` directory. File naming convention:

- `feed_123.toml` (for feed ID 123)

#### Basic Rule Structure

```toml
feed_id = 123
enabled = true              # Optional, defaults to true

[[rules]]
action = "markread"         # Currently only "markread" is supported

[[rules.conditions]]
field = "title"             # "title", "content", "author", or "url"
operator = "contains"       # See operators below
value = "advertisement"
```

#### Available Operators

- `contains` / `notcontains`: Case-insensitive substring matching
- `equals` / `notequals`: Case-insensitive exact matching
- `startswith`: Case-insensitive prefix matching
- `endswith`: Case-insensitive suffix matching
- `matches`: Regular expression matching (case-sensitive)

#### Example Rule File

```toml
feed_id = 123
enabled = true

# Block promotional content
[[rules]]
action = "markread"

[[rules.conditions]]
field = "title"
operator = "contains"
value = "sponsored"

[[rules.conditions]]
field = "title"
operator = "contains"
value = "advertisement"

# Block specific authors
[[rules]]
action = "markread"

[[rules.conditions]]
field = "author"
operator = "equals"
value = "Marketing Team"
```

## Finding Feed IDs

The web interface automatically displays feed information, or you can:

1. Log into your Miniflux web interface
2. Navigate to a feed
3. Check the URL: `https://your-miniflux.com/feeds/123` (123 is the feed ID)
4. Or use the Miniflux API: `GET /v1/feeds`
