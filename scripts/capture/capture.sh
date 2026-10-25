#!/bin/bash
# Automated screenshot and demo GIF capture for the Shipyard README.
#
# Runs the installed Shipyard.app against throwaway demo repositories and an
# isolated HOME, drives it with coordinate clicks, and writes:
#   docs/screenshots/01-dashboard.png ... 05-settings.png
#   docs/demo.gif
#
# Requirements (macOS, Apple Silicon):
#   - /Applications/Shipyard.app
#   - ffmpeg (brew install ffmpeg)
#   - Accessibility and Screen Recording permission for the app running this
#     script (Terminal, Cursor, ...). See scripts/capture/README.md.
#
# Usage:
#   scripts/capture/capture.sh [--dry-run] [--only <name>]
#   names: 01-dashboard 02-history 03-working-tree 04-branches 05-settings
#
# Safety:
#   - Your real Shipyard keeps running and is never quit or killed. Every action
#     targets only the instance started here, by process id.
#   - Your real settings and repositories are untouched. HOME is redirected for
#     the app's data directory, and the few webview keys that cannot be
#     redirected are removed again (see localstorage.py).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SCREENSHOTS_DIR="$PROJECT_ROOT/docs/screenshots"
DEMO_GIF_PATH="$PROJECT_ROOT/docs/demo.gif"

APP_BINARY="/Applications/Shipyard.app/Contents/MacOS/Shipyard"
DEMO_DIR="/tmp/shipyard-demo-capture"   # keys mentioning this are purged from webview storage
ISOLATED_HOME="/tmp/shipyard-capture-home"

WINDOW_WIDTH=1440
WINDOW_HEIGHT=900
WINDOW_Y=40
REF_WIDTH=1024   # click coordinates below are in a 1024x640 reference space
REF_HEIGHT=640

DRY_RUN=false
ONLY_SHOT=""
APP_PID=""
RECORD_PID=""

# ---------------------------------------------------------------- output ----
BLUE=$'\033[34m'; GREEN=$'\033[32m'; RED=$'\033[31m'; YELLOW=$'\033[33m'; NC=$'\033[0m'
info()    { echo "${BLUE}▸${NC} $*"; }
success() { echo "${GREEN}✓${NC} $*"; }
warn()    { echo "${YELLOW}!${NC} $*" >&2; }
die()     { echo "${RED}✗${NC} $*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=true; shift ;;
    --only) ONLY_SHOT="${2:-}"; shift 2 ;;
    --only=*) ONLY_SHOT="${1#*=}"; shift ;;
    -h|--help) sed -n '2,28p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) die "Unknown option: $1" ;;
  esac
done

# --------------------------------------------------------------- cleanup ----
cleanup() {
  local code=$?
  [[ -n "$RECORD_PID" ]] && kill -INT "$RECORD_PID" 2>/dev/null || true
  # Quit only the instance this script started.
  if [[ -n "$APP_PID" ]] && kill -0 "$APP_PID" 2>/dev/null; then
    kill "$APP_PID" 2>/dev/null || true
  fi
  python3 "$SCRIPT_DIR/localstorage.py" purge-demo 2>/dev/null || true
  rm -rf "$ISOLATED_HOME" "$DEMO_DIR" 2>/dev/null || true
  exit $code
}
trap cleanup EXIT

# --------------------------------------------------------- prerequisites ----
[[ -x "$APP_BINARY" ]] || die "Shipyard.app not found at /Applications/Shipyard.app"
command -v ffmpeg >/dev/null 2>&1 || die "ffmpeg is required: brew install ffmpeg"
command -v python3 >/dev/null 2>&1 || die "python3 is required"

# The app's webview storage is shared with a running real Shipyard. It is only
# touched through localstorage.py (demo keys only), so running alongside is safe.
if pgrep -xi shipyard >/dev/null 2>&1; then
  warn "Shipyard is already running. It will not be touched; only the instance started here is controlled."
fi

# ------------------------------------------------------- app and demo data --
python3 "$SCRIPT_DIR/localstorage.py" purge-demo || true

info "Generating demo repositories in $DEMO_DIR"
rm -rf "$DEMO_DIR" "$ISOLATED_HOME"
bash "$SCRIPT_DIR/generate-demo-data.sh" "$DEMO_DIR" >/dev/null
bash "$SCRIPT_DIR/seed-app-data.sh" "$DEMO_DIR" "$ISOLATED_HOME" >/dev/null
success "Demo data ready"

if [[ "$DRY_RUN" == true ]]; then
  warn "Dry run: not launching the app or capturing anything"
  exit 0
fi

# Screen size decides where the 1440x900 window can sit.
read -r SCREEN_W SCREEN_H < <(osascript -l JavaScript -e 'ObjC.import("AppKit"); var f=$.NSScreen.mainScreen.frame; Math.round(f.size.width)+" "+Math.round(f.size.height)')
(( SCREEN_W >= WINDOW_WIDTH + 24 && SCREEN_H >= WINDOW_HEIGHT + WINDOW_Y + 20 )) \
  || die "Screen is ${SCREEN_W}x${SCREEN_H}; need at least $((WINDOW_WIDTH + 24))x$((WINDOW_HEIGHT + WINDOW_Y + 20)) for a ${WINDOW_WIDTH}x${WINDOW_HEIGHT} window"
WINDOW_X=$(( (SCREEN_W - WINDOW_WIDTH) / 2 ))

info "Launching Shipyard with an isolated HOME"
HOME="$ISOLATED_HOME" "$APP_BINARY" >/dev/null 2>&1 &
APP_PID=$!

# ------------------------------------------------------------- helpers ------
# Window id of the main window of the instance we started (by pid, so a real
# running Shipyard is never picked).
window_id() {
  osascript -l JavaScript -e "ObjC.import('CoreGraphics');
    var l=ObjC.deepUnwrap(ObjC.castRefToObject(\$.CGWindowListCopyWindowInfo(\$.kCGWindowListOptionOnScreenOnly,0)));
    l.filter(w=>w.kCGWindowOwnerPID==$APP_PID&&w.kCGWindowLayer==0&&w.kCGWindowBounds.Width>500).map(w=>w.kCGWindowNumber).join('')" 2>/dev/null
}

app_sys_events() {  # run an AppleScript snippet "inside" our process
  osascript -e "tell application \"System Events\" to tell (first process whose unix id is $APP_PID) to $1" 2>/dev/null
}

bring_to_front() {
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $APP_PID) to true" 2>/dev/null || true
    [[ "$(osascript -e 'tell application "System Events" to get unix id of first process whose frontmost is true' 2>/dev/null)" == "$APP_PID" ]] && return 0
    sleep 0.3
  done
  die "Could not bring Shipyard to the front"
}

# Click at a point in the 1024x640 reference space of the window.
click_ref() {
  local x=$(( WINDOW_X + $1 * WINDOW_WIDTH / REF_WIDTH ))
  local y=$(( WINDOW_Y + $2 * WINDOW_HEIGHT / REF_HEIGHT ))
  bring_to_front
  osascript -e "tell application \"System Events\" to click at {$x, $y}" >/dev/null 2>&1 || true
  sleep "${3:-1}"
}

# Park the pointer somewhere harmless so no row or button shows a hover state.
park_pointer() { click_ref 640 600 0.4; }

capture() {
  local name="$1"
  [[ -n "$ONLY_SHOT" && "$ONLY_SHOT" != "$name" ]] && return 0
  mkdir -p "$SCREENSHOTS_DIR"
  local wid; wid="$(window_id)"
  [[ -n "$wid" ]] || die "No Shipyard window found for pid $APP_PID"
  screencapture -l "$wid" -o "$SCREENSHOTS_DIR/$name.png" \
    || die "screencapture failed (is Screen Recording allowed for this app?)"
  success "Captured $name.png"
}

# --------------------------------------------------------------- window -----
for _ in $(seq 1 40); do [[ -n "$(window_id)" ]] && break; sleep 0.5; done
[[ -n "$(window_id)" ]] || die "Shipyard window did not appear"
sleep 2
app_sys_events "tell window 1 to set position to {$WINDOW_X, $WINDOW_Y}"
app_sys_events "tell window 1 to set size to {$WINDOW_WIDTH, $WINDOW_HEIGHT}"
sleep 1.5
bring_to_front
success "Window ready (${WINDOW_WIDTH}x${WINDOW_HEIGHT} at $WINDOW_X,$WINDOW_Y)"

# The first click after launch can be swallowed while the window activates.
click_ref 640 600 0.5
click_ref 640 600 0.5

# ----------------------------------------------------------- screenshots ----
# 1. Dashboard with groups
park_pointer
capture 01-dashboard

# 2. Commit history with branch graph (open the first repository)
click_ref 137 211 2.5
park_pointer
capture 02-history

# 3. Working tree with a real diff (first unstaged file)
click_ref 860 114 2
park_pointer
capture 03-working-tree

# 4. Branches
click_ref 30 67 1.5     # Back
click_ref 108 90 1.5    # Branches tab
park_pointer
capture 04-branches

# 5. Settings (gear, top right)
click_ref 1011 36 2
park_pointer
capture 05-settings

# ----------------------------------------------------------------- GIF ------
if [[ -z "$ONLY_SHOT" ]]; then
  info "Recording the demo GIF"
  # Close the settings and repository tabs, then start from the dashboard.
  click_ref 307 36 1
  click_ref 224 36 1
  click_ref 62 36 1.5

  TEMP_VIDEO="/tmp/shipyard-demo-$$.mov"
  PALETTE="/tmp/shipyard-palette-$$.png"
  rm -f "$TEMP_VIDEO"
  screencapture -v -R"$WINDOW_X,$WINDOW_Y,$WINDOW_WIDTH,$WINDOW_HEIGHT" "$TEMP_VIDEO" 2>/dev/null &
  RECORD_PID=$!
  sleep 1.5

  click_ref 137 211 2.5    # open web-app-project
  click_ref 330 260 2.5    # a commit
  click_ref 30 67 1.5      # back
  click_ref 860 114 2.5    # an unstaged file diff
  click_ref 30 67 1.5      # back
  click_ref 108 90 2.5     # branches
  click_ref 180 90 2       # tags
  click_ref 62 36 2.5      # back to the dashboard

  kill -INT "$RECORD_PID" 2>/dev/null || true
  wait "$RECORD_PID" 2>/dev/null || true
  RECORD_PID=""
  sleep 1
  [[ -s "$TEMP_VIDEO" ]] || die "Recording failed (is Screen Recording allowed for this app?)"

  ffmpeg -loglevel error -y -i "$TEMP_VIDEO" -vf "fps=12,scale=960:-1:flags=lanczos,palettegen=stats_mode=diff" "$PALETTE"
  ffmpeg -loglevel error -y -i "$TEMP_VIDEO" -i "$PALETTE" \
    -lavfi "fps=12,scale=960:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=5" "$DEMO_GIF_PATH"
  rm -f "$TEMP_VIDEO" "$PALETTE"

  size_bytes=$(stat -f%z "$DEMO_GIF_PATH")
  success "Demo GIF created: docs/demo.gif ($((size_bytes / 1024)) KB)"
  (( size_bytes > 8388608 )) && warn "GIF is over 8 MB; shorten the flow or lower the fps"
fi

success "Done. Review docs/screenshots/*.png and docs/demo.gif, then commit them."
