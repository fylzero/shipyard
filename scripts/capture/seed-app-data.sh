#!/bin/bash
set -euo pipefail

# Seed demo repositories into Shipyard's isolated data directory
# This populates the settings.json file that Shipyard reads on launch

DEMO_DIR="${1:-}"
ISOLATED_HOME="${2:-}"

if [[ -z "$DEMO_DIR" || -z "$ISOLATED_HOME" ]]; then
  echo "Usage: $0 <demo-repos-dir> <isolated-home-dir>" >&2
  exit 1
fi

info() { echo "$(tput setaf 4)ℹ$(tput sgr0) $*"; }
success() { echo "$(tput setaf 2)✓$(tput sgr0) $*"; }
error() { echo "$(tput setaf 1)✗$(tput sgr0) $*" >&2; }

# Tauri app data directory for macOS
# Based on tauri::path::app_data_dir() which uses directories::BaseDirs::data_dir()
# For macOS: ~/Library/Application Support/{bundle_identifier}
APP_DATA_DIR="$ISOLATED_HOME/Library/Application Support/com.shipyard.app"

info "Creating isolated app data directory: $APP_DATA_DIR"
mkdir -p "$APP_DATA_DIR"

# Generate settings.json with the demo repositories
# The app expects a specific JSON structure based on models.rs:
# - groups: array of repository groups
# - repos: array of standalone repositories
# - dashboardOrder: ordered list of group/repo IDs

cat > "$APP_DATA_DIR/settings.json" << EOF
{
  "groups": [
    {
      "id": "demo-group-1",
      "name": "Development",
      "expanded": true,
      "pullFromBranch": "main",
      "checkoutFallbacks": ["main", "develop"],
      "headerColor": "#2d5f7f",
      "repos": [
        {
          "id": "web-app-project",
          "path": "$DEMO_DIR/web-app-project",
          "label": "Web App",
          "headerColor": ""
        },
        {
          "id": "api-server",
          "path": "$DEMO_DIR/api-server",
          "label": "API Server",
          "headerColor": ""
        },
        {
          "id": "mobile-client",
          "path": "$DEMO_DIR/mobile-client",
          "label": "Mobile",
          "headerColor": ""
        }
      ]
    },
    {
      "id": "demo-group-2",
      "name": "Infrastructure",
      "expanded": true,
      "pullFromBranch": "main",
      "checkoutFallbacks": ["main"],
      "headerColor": "#3f6b4f",
      "repos": [
        {
          "id": "infra-config",
          "path": "$DEMO_DIR/infra-config",
          "label": "Infra",
          "headerColor": ""
        },
        {
          "id": "deploy-tools",
          "path": "$DEMO_DIR/deploy-tools",
          "label": "Deploy",
          "headerColor": ""
        }
      ]
    }
  ],
  "repos": [
    {
      "id": "docs-site",
      "path": "$DEMO_DIR/docs-site",
      "label": "Documentation",
      "headerColor": "#5f4d2d"
    }
  ],
  "dashboardOrder": [
    "demo-group-1",
    "demo-group-2",
    "docs-site"
  ],
  "refreshIntervalSeconds": 300,
  "filesPaneWidth": 320,
  "terminalPaneHeight": 280,
  "diffMode": "split",
  "diffFontFamily": "jetbrains",
  "diffFontSize": 13.0,
  "terminalFontFamily": "jetbrains",
  "terminalFontSize": 14.0,
  "editor": "system",
  "notifications": "off",
  "refreshActiveHours": {
    "enabled": false,
    "preset": "business",
    "start": "08:00",
    "end": "18:00"
  }
}
EOF

success "Seeded app data: $APP_DATA_DIR/settings.json"
info "Repositories configured:"
echo "  - Group 'Development': web-app-project, api-server"
echo "  - Standalone: docs-site"
