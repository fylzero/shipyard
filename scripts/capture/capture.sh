#!/bin/bash
set -euo pipefail

# Shipyard Screenshot and Demo Capture Script
# Automates capturing screenshots and demo GIF for the README
#
# Prerequisites:
#   - macOS Screen Recording permission for Terminal
#   - macOS Accessibility permission for Terminal
#   - Shipyard.app installed at /Applications/Shipyard.app
#   - ffmpeg installed (for GIF conversion)
#
# Usage:
#   ./capture.sh [--dry-run] [--only <screenshot-name>]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DOCS_DIR="$PROJECT_ROOT/docs"
SCREENSHOTS_DIR="$DOCS_DIR/screenshots"

APP_PATH="/Applications/Shipyard.app"
BUNDLE_ID="com.shipyard.app"
APP_BINARY="$APP_PATH/Contents/MacOS/Shipyard"

WINDOW_WIDTH=1440
WINDOW_HEIGHT=900

DRY_RUN=false
ONLY_SCREENSHOT=""

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --only)
      ONLY_SCREENSHOT="$2"
      shift 2
      ;;
    *)
      echo "Unknown option: $1" >&2
      echo "Usage: $0 [--dry-run] [--only <screenshot-name>]" >&2
      exit 1
      ;;
  esac
done

# Color output
info() { echo "$(tput setaf 4)ℹ$(tput sgr0) $*"; }
success() { echo "$(tput setaf 2)✓$(tput sgr0) $*"; }
error() { echo "$(tput setaf 1)✗$(tput sgr0) $*" >&2; }
warn() { echo "$(tput setaf 3)⚠$(tput sgr0) $*" >&2; }

# Verify prerequisites
check_prerequisites() {
  info "Checking prerequisites..."
  
  if [[ ! -d "$APP_PATH" ]]; then
    error "Shipyard.app not found at $APP_PATH"
    exit 1
  fi
  
  if ! command -v ffmpeg &> /dev/null; then
    error "ffmpeg is required but not installed"
    error "Install it with: brew install ffmpeg"
    exit 1
  fi
  
  success "Prerequisites satisfied"
}

# Clean up on exit
cleanup() {
  info "Cleaning up..."
  
  # Quit the app if running
  osascript -e "tell application \"Shipyard\" to quit" 2>/dev/null || true
  
  # Give it a moment to quit gracefully
  sleep 1
  
  # Force kill if still running
  pkill -9 -f "$APP_BINARY" 2>/dev/null || true
  
  # Clean up temporary directories
  if [[ -n "${ISOLATED_HOME:-}" && -d "$ISOLATED_HOME" ]]; then
    rm -rf "$ISOLATED_HOME"
  fi
  
  if [[ -n "${DEMO_DIR:-}" && -d "$DEMO_DIR" ]]; then
    rm -rf "$DEMO_DIR"
  fi
}

trap cleanup EXIT

# Generate demo data
generate_demo_data() {
  info "Generating demo Git repositories..."
  
  DEMO_DIR=$(mktemp -d -t shipyard-demo)
  bash "$SCRIPT_DIR/generate-demo-data.sh" "$DEMO_DIR"
  
  success "Demo data generated in $DEMO_DIR"
}

# Set up isolated environment
setup_isolated_environment() {
  info "Setting up isolated app environment..."
  
  ISOLATED_HOME=$(mktemp -d -t shipyard-home)
  bash "$SCRIPT_DIR/seed-app-data.sh" "$DEMO_DIR" "$ISOLATED_HOME"
  
  success "Isolated environment ready at $ISOLATED_HOME"
}

# Launch the app with isolated environment
launch_app() {
  info "Launching Shipyard with isolated environment..."
  
  # Launch with custom HOME to isolate app data
  HOME="$ISOLATED_HOME" "$APP_BINARY" &> /dev/null &
  APP_PID=$!
  
  info "App launched with PID $APP_PID"
  
  # Wait for app to be ready
  info "Waiting for app window to appear..."
  local max_wait=30
  local waited=0
  
  while [[ $waited -lt $max_wait ]]; do
    if swift "$SCRIPT_DIR/find-window-id.swift" "$BUNDLE_ID" &> /dev/null; then
      success "App window ready"
      sleep 2 # Extra settling time
      return 0
    fi
    sleep 1
    waited=$((waited + 1))
  done
  
  error "App window did not appear within ${max_wait}s"
  return 1
}

# Get window ID
get_window_id() {
  swift "$SCRIPT_DIR/find-window-id.swift" "$BUNDLE_ID"
}

# Position and resize the window
setup_window() {
  local window_id
  window_id=$(get_window_id)
  
  info "Setting window size to ${WINDOW_WIDTH}x${WINDOW_HEIGHT} (window ID: $window_id)"
  
  # Use osascript to resize and position the window
  osascript <<EOF
tell application "System Events"
  tell process "Shipyard"
    set frontmost to true
    tell window 1
      set position to {100, 100}
      set size to {$WINDOW_WIDTH, $WINDOW_HEIGHT}
    end tell
  end tell
end tell
EOF
  
  sleep 1
  success "Window positioned and resized"
}

# Capture a screenshot
capture_screenshot() {
  local name="$1"
  local filename="$2"
  
  if [[ "$DRY_RUN" == "true" ]]; then
    info "[DRY RUN] Would capture: $name -> $filename"
    return 0
  fi
  
  if [[ -n "$ONLY_SCREENSHOT" && "$name" != "$ONLY_SCREENSHOT" ]]; then
    return 0
  fi
  
  info "Capturing: $name"
  
  local window_id
  window_id=$(get_window_id)
  
  local output_path="$SCREENSHOTS_DIR/$filename"
  
  # Use screencapture with window ID to capture without shadow
  screencapture -l "$window_id" -o -x "$output_path"
  
  if [[ -f "$output_path" ]]; then
    local size
    size=$(du -h "$output_path" | cut -f1)
    success "Captured $name ($size): $filename"
  else
    error "Failed to capture $name"
    return 1
  fi
}

# Navigate using AppleScript/System Events
navigate_to() {
  local target="$1"
  
  info "Navigating to: $target"
  
  case "$target" in
    dashboard)
      # Click on "Repositories" tab or press Cmd+1
      osascript <<'EOF'
tell application "System Events"
  tell process "Shipyard"
    keystroke "1" using command down
  end tell
end tell
EOF
      ;;
      
    repo-first)
      # Click on the first repository to open it
      osascript <<'EOF'
tell application "System Events"
  tell process "Shipyard"
    -- Find the first repo row in the groups list and click it
    try
      click button 1 of group 1 of group 1 of group 1 of group 1 of window 1
    on error
      -- Fallback: try double-clicking in the repo area
      click at {400, 400}
    end try
  end tell
end tell
EOF
      ;;
      
    history)
      # Open history view - look for menu item or try keyboard shortcut
      osascript <<'EOF'
tell application "System Events"
  tell process "Shipyard"
    set frontmost to true
    try
      click menu item "History" of menu 1 of menu bar item "Window" of menu bar 1
    on error
      -- Fallback: try clicking the History button in status bar if visible
      log "Could not find History menu item"
    end try
  end tell
end tell
EOF
      ;;
      
    branches)
      # Within a repo view, click on branches tab/section
      osascript <<'EOF'
tell application "System Events"
  tell process "Shipyard"
    keystroke "b" using {command down, shift down}
  end tell
end tell
EOF
      ;;
      
    settings)
      # Open settings with Cmd+,
      osascript <<'EOF'
tell application "System Events"
  tell process "Shipyard"
    keystroke "," using command down
  end tell
end tell
EOF
      ;;
      
    *)
      warn "Unknown navigation target: $target"
      ;;
  esac
  
  sleep 2 # Wait for UI to settle
}

# Click at specific coordinates (relative to window)
click_at() {
  local x="$1"
  local y="$2"
  
  osascript <<EOF
tell application "System Events"
  tell process "Shipyard"
    set frontmost to true
    click at {$x, $y}
  end tell
end tell
EOF
  
  sleep 1
}

# Capture all screenshots following the README's TODO list
capture_all_screenshots() {
  info "Starting screenshot capture sequence..."
  
  mkdir -p "$SCREENSHOTS_DIR"
  
  # Screenshot 1: Dashboard view with multiple repository groups
  navigate_to dashboard
  capture_screenshot "01-dashboard" "01-dashboard.png"
  
  # Screenshot 2: Commit history with visual graph and file changes
  # First, open a repository
  navigate_to repo-first
  sleep 2
  
  # Click on a commit in the history list to show file changes
  click_at 300 400
  sleep 1
  
  capture_screenshot "02-history" "02-history.png"
  
  # Screenshot 3: Working tree with staged/unstaged changes and diff viewer
  # The demo repos have working tree changes
  # Click on the "Changes" or working tree tab
  click_at 200 300
  sleep 1
  
  capture_screenshot "03-working-tree" "03-working-tree.png"
  
  # Screenshot 4: Branch list with tracking information
  navigate_to branches
  capture_screenshot "04-branches" "04-branches.png"
  
  # Screenshot 5: Repository settings showing group configuration
  navigate_to dashboard
  sleep 1
  
  # Right-click on a group to open settings (if context menu available)
  # Or use a settings button
  click_at 1350 200 # Settings gear icon area
  sleep 1
  
  capture_screenshot "05-settings" "05-settings.png"
  
  success "All screenshots captured!"
}

# Record demo video and convert to GIF
record_demo() {
  if [[ "$DRY_RUN" == "true" ]]; then
    info "[DRY RUN] Would record demo video"
    return 0
  fi
  
  if [[ -n "$ONLY_SCREENSHOT" ]]; then
    info "Skipping demo recording (--only specified)"
    return 0
  fi
  
  info "Recording demo video (10-15 seconds)..."
  
  local temp_video="/tmp/shipyard-demo-$$.mov"
  local output_gif="$DOCS_DIR/demo.gif"
  
  # Navigate to dashboard to start
  navigate_to dashboard
  sleep 1
  
  # Start recording with screencapture
  info "Recording started - performing demo actions..."
  
  local window_id
  window_id=$(get_window_id)
  
  # Start screen recording in background
  # Note: screencapture -V requires the window to be visible
  # Record for ~12 seconds
  timeout 12s screencapture -v -V "$temp_video" 2>/dev/null || true &
  local record_pid=$!
  
  sleep 2
  
  # Perform demo actions
  # Action 1: Switch repos
  click_at 300 300
  sleep 2
  
  # Action 2: View some changes
  click_at 250 350
  sleep 2
  
  # Action 3: Navigate to history
  navigate_to history
  sleep 2
  
  # Action 4: Back to dashboard
  navigate_to dashboard
  sleep 2
  
  # Wait for recording to finish
  wait $record_pid || true
  
  if [[ ! -f "$temp_video" ]]; then
    error "Demo video recording failed"
    return 1
  fi
  
  success "Demo video recorded: $temp_video"
  
  # Convert to optimized GIF using ffmpeg with palettegen
  info "Converting to optimized GIF (target: ~8 MB, 960px wide)..."
  
  local palette="/tmp/palette-$$.png"
  
  # Generate palette
  ffmpeg -i "$temp_video" -vf "fps=15,scale=960:-1:flags=lanczos,palettegen=stats_mode=diff" -y "$palette" &> /dev/null
  
  # Generate GIF with palette
  ffmpeg -i "$temp_video" -i "$palette" -filter_complex "fps=15,scale=960:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=5" -y "$output_gif" &> /dev/null
  
  # Clean up
  rm -f "$temp_video" "$palette"
  
  if [[ -f "$output_gif" ]]; then
    local size
    size=$(du -h "$output_gif" | cut -f1)
    success "Demo GIF created: demo.gif ($size)"
    
    # Check if it's over 8 MB
    local size_mb
    size_mb=$(du -m "$output_gif" | cut -f1)
    if [[ $size_mb -gt 8 ]]; then
      warn "GIF size ($size) exceeds 8 MB target"
    fi
  else
    error "Failed to create demo GIF"
    return 1
  fi
}

# Main execution
main() {
  info "Shipyard Screenshot Capture Tool"
  info "==============================="
  echo
  
  check_prerequisites
  
  if [[ "$DRY_RUN" == "true" ]]; then
    warn "Running in DRY RUN mode - no actual captures will be performed"
  fi
  
  if [[ -n "$ONLY_SCREENSHOT" ]]; then
    info "Only capturing screenshot: $ONLY_SCREENSHOT"
  fi
  
  generate_demo_data
  setup_isolated_environment
  launch_app
  setup_window
  
  capture_all_screenshots
  record_demo
  
  success "All done! Screenshots saved to: $SCREENSHOTS_DIR"
  success "Demo GIF saved to: $DOCS_DIR/demo.gif"
}

main "$@"
