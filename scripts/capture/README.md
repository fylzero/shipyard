# Screenshot and Demo Capture Automation

This directory contains scripts to automatically generate the screenshots and demo GIF for Shipyard's README.

## Overview

The capture tooling runs on macOS against the installed Shipyard.app at `/Applications/Shipyard.app`. It creates isolated demo Git repositories with realistic history, seeds them into a temporary app data directory, launches Shipyard in isolation, and drives the UI with coordinate-based clicks to capture screenshots and a demo video.

## Prerequisites

### macOS Permissions

The Terminal app (or whichever terminal you run the script from) must have:

1. **Screen Recording** permission
   - System Settings → Privacy & Security → Screen Recording
   - Enable permission for Terminal (or iTerm, Cursor, etc.)

2. **Accessibility** permission
   - System Settings → Privacy & Security → Accessibility
   - Enable permission for Terminal (or iTerm, Cursor, etc.)

These permissions allow the scripts to control Shipyard's UI and capture screenshots.

### Software Requirements

- **macOS** (Apple Silicon or Intel)
- **Shipyard.app** installed at `/Applications/Shipyard.app`
- **ffmpeg** for GIF conversion: `brew install ffmpeg`
- **python3** (pre-installed on macOS)
- **Screen resolution**: At least ~1440×940 pixels

The scripts use standard macOS tools: `bash`, `osascript`, `screencapture`, `python3`.

## How It Works

### 1. Demo Data Generation (`generate-demo-data.sh`)

Creates three throwaway Git repositories in `/tmp/shipyard-demo-capture/` with realistic invented history:

- **web-app-project**: ~30 commits, 3 branches, 2 tags, working tree changes
- **api-server**: ~40 commits, 4 branches, 2 tags, 1 stash
- **docs-site**: ~20 commits, 2 branches, 1 tag

Each repository includes:
- Multiple commits from fake authors (invented names, `@example.local` emails)
- Fake git identity configured with `GIT_CONFIG_GLOBAL=/dev/null`
- Monotonic commit dates (oldest to newest)
- Branch history with merges and upstream tracking
- Tags (annotated, without the invalid `--quiet` flag)
- A bare remote for ahead/behind states
- Working tree with staged, unstaged, and untracked files

**All names and content are invented.** No real people, no company names, no product references.

### 2. App Data Isolation (`seed-app-data.sh`)

Creates an isolated app data directory at `/tmp/shipyard-capture-home/` and populates `settings.json` with the demo repositories:
- One repository group ("Development") containing web-app-project and api-server
- One standalone repository (docs-site)

The app binary is launched with `HOME=/tmp/shipyard-capture-home`, redirecting Tauri's `app_data_dir()` to the isolated directory.

**Note on WebKit localStorage:** The app's webview keeps per-repository UI state in `~/Library/WebKit/com.shipyard.app/`, which is not redirected by the custom `HOME`. The `localstorage.py` helper purges only keys containing `shipyard-demo-capture` on cleanup, leaving the user's real app state untouched.

### 3. Main Capture Script (`capture.sh`)

The orchestration script:

1. **Warns if Shipyard is already running** - your real instance is never touched or killed
2. **Generates demo data** in `/tmp/shipyard-demo-capture/`
3. **Launches Shipyard** with `HOME=/tmp/shipyard-capture-home` to isolate app data
4. **Tracks the new process by PID** - all actions target only this instance
5. **Positions and resizes** the window to 1440×900 at position (100, 40)
6. **Performs warm-up clicks** - the first click after launch is swallowed by macOS, so the script sends a few initial clicks before real navigation
7. **Navigates through the UI** using AppleScript coordinate clicks:
   - Coordinates are defined in a 1024×640 reference space
   - Scaled to the actual 1440×900 window at runtime
   - Dashboard view
   - Repository detail with commit history
   - Working tree with file changes
   - Branch list
   - Settings/group configuration
8. **Captures each screenshot** with `screencapture` using the app's PID
9. **Records a ~12 second demo** showing repository navigation and UI interaction
10. **Converts the video to GIF** using ffmpeg with palettegen/paletteuse (15 fps, 960px wide)
11. **Cleans up**: Quits only the launched instance, purges demo keys from localStorage, removes temp directories

The five screenshots:
1. `01-dashboard.png` - Dashboard view with multiple repository groups
2. `02-history.png` - Commit history with visual graph and file changes
3. `03-working-tree.png` - Working tree with staged/unstaged changes and diff viewer
4. `04-branches.png` - Branch list with tracking information
5. `05-settings.png` - Repository settings showing group configuration

### 4. localStorage Cleanup (`localstorage.py`)

A Python helper that purges only demo-related keys (matching `%shipyard-demo-capture%`) from the webview's SQLite localStorage database in `~/Library/WebKit/com.shipyard.app/`. This prevents leftover state from demo repositories appearing in the user's real app.

## Usage

### Capture All Screenshots and Demo

Run the full capture sequence:

```bash
cd scripts/capture
bash capture.sh
```

This will:
- Generate demo data
- Launch Shipyard in isolation
- Capture all 5 screenshots
- Record and convert the demo GIF
- Clean up and save everything to `docs/screenshots/` and `docs/`

### Dry Run

Preview what would be captured without launching the app:

```bash
bash capture.sh --dry-run
```

### Capture a Single Screenshot

Retry just one screenshot (useful if one failed or needs adjustment):

```bash
bash capture.sh --only 03-working-tree
```

Available screenshot names:
- `01-dashboard`
- `02-history`
- `03-working-tree`
- `04-branches`
- `05-settings`

### After Capture

The script saves:
- `docs/screenshots/*.png` - 5 screenshot images
- `docs/demo.gif` - Demo animation

Review these files, then commit and push them.

## Troubleshooting

### "Shipyard.app not found at /Applications/Shipyard.app"

- Verify Shipyard is installed at `/Applications/Shipyard.app`
- Check that the app binary exists at `/Applications/Shipyard.app/Contents/MacOS/Shipyard`

### "Operation not permitted" when using System Events

- Grant **Accessibility** permission to Terminal
- System Settings → Privacy & Security → Accessibility → Add Terminal
- Restart Terminal after granting permission

### Screenshots are blank or wrong size

- Ensure your screen resolution is at least ~1440×940 pixels
- The window must be fully visible and not obstructed
- Try increasing `sleep` delays in the script if the UI hasn't settled

### GIF size exceeds 8 MB

- Reduce frame rate (currently 15 fps) in the ffmpeg command
- Reduce width (currently 960px)
- Shorten the recording duration
- Increase `bayer_scale` for more aggressive dithering

### Navigation fails or clicks miss targets

- The script uses a 1024×640 reference coordinate space scaled to 1440×900
- If the UI layout changes, adjust click coordinates in `capture.sh`
- Add more `sleep` delays between actions if the UI is slow to respond

### Demo keys remain in localStorage

- Run `python3 localstorage.py purge-demo` manually
- Check that the script has write access to `~/Library/WebKit/com.shipyard.app/`

## Refreshing After UI Changes

After making changes to Shipyard's UI that affect the screenshots:

1. Build and install the new version: `bun run tauri build` and copy to `/Applications/`
2. Run the capture script: `bash scripts/capture/capture.sh`
3. Review the new screenshots and demo
4. Commit and push the updated images

## Script Architecture

```
scripts/capture/
├── capture.sh                 # Main orchestration script (PID-based automation)
├── generate-demo-data.sh      # Creates demo Git repositories
├── seed-app-data.sh           # Populates isolated settings.json
├── localstorage.py            # Purges demo keys from WebKit localStorage
└── README.md                  # This file
```

All scripts:
- Use `set -euo pipefail` for safety
- Provide colored progress output
- Clean up temporary files on exit
- Print clear errors with context

## Safety

- **Your real Shipyard keeps running** - The script tracks only the instance it starts by process ID and never kills or quits your real app
- **Your settings and repositories are untouched** - App data is isolated via custom `HOME`
- **WebKit localStorage is cleaned** - Only demo keys matching `%shipyard-demo-capture%` are removed; your real app state is preserved

## Limitations

- **macOS only** - These scripts cannot run on Linux or Windows
- **Requires installed app** - Cannot run against `bun run tauri dev`
- **Coordinate-based automation** - Click coordinates may need adjustment if UI layout changes significantly
- **Timing-dependent** - Fixed `sleep` delays may be insufficient on slower machines
- **Minimum screen resolution** - Requires at least ~1440×940 pixels to fit the window
