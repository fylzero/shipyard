# Screenshot and Demo Capture Automation

This directory contains scripts to automatically generate the screenshots and demo GIF for Shipyard's README.

## Overview

The capture tooling runs on macOS against the installed Shipyard.app at `/Applications/Shipyard.app` (version 1.16.0). It creates isolated demo Git repositories with realistic history, seeds them into a temporary app data directory, launches Shipyard in isolation, and drives the UI to capture screenshots and a demo video.

## Prerequisites

### macOS Permissions

The Terminal app (or whichever terminal you run the script from) must have:

1. **Screen Recording** permission
   - Go to System Settings → Privacy & Security → Screen Recording
   - Enable permission for Terminal (or iTerm, etc.)

2. **Accessibility** permission
   - Go to System Settings → Privacy & Security → Accessibility
   - Enable permission for Terminal (or iTerm, etc.)

These permissions allow the scripts to:
- Control Shipyard's UI via AppleScript/System Events
- Capture window screenshots with `screencapture`
- Record screen video for the demo GIF

### Software Requirements

- **macOS** (Apple Silicon or Intel)
- **Shipyard.app** installed at `/Applications/Shipyard.app`
- **ffmpeg** for GIF conversion: `brew install ffmpeg`
- **Git** (already required by Shipyard)
- **Bun** (for development)

The scripts use standard macOS tools:
- `bash`, `osascript`, `screencapture`, `swift`
- No additional dependencies beyond `ffmpeg`

## How It Works

### 1. Demo Data Generation (`generate-demo-data.sh`)

Creates three throwaway Git repositories in a temporary directory with realistic invented history:

- **web-app-project**: Web application with ~30 commits, 3 branches (main, feature/user-dashboard, feature/auth-system, develop), 2 tags, staged/unstaged/untracked changes
- **api-server**: API server with ~40 commits, 4 branches, 2 tags, 1 stash, working tree changes
- **docs-site**: Documentation site with ~20 commits, 2 branches, 1 tag, staged changes

Each repository includes:
- Multiple commits from fake authors (invented names, `@example.local` emails)
- Branch history with merges
- Tags (annotated)
- A bare remote for ahead/behind states
- Working tree with staged, unstaged, and untracked files

**All names and content are invented.** No real people, no company names, no product references.

### 2. App Data Isolation (`seed-app-data.sh`)

Creates an isolated app data directory and populates `settings.json` with the demo repositories:
- One repository group ("Development") containing web-app-project and api-server
- One standalone repository (docs-site)

This ensures:
- No interference with the maintainer's real Shipyard data
- Clean, repeatable captures
- No credentials or private data exposure

**Data storage determination:** Shipyard is a Tauri app that calls `app_data_dir()` (from `src-tauri/src/persist.rs` line 15), which resolves to `~/Library/Application Support/com.shipyard.app/` on macOS via the directories crate. The app stores its repository list, groups, and settings in `settings.json` in that directory. By launching the app with a custom `HOME` environment variable, we redirect all app data to an isolated temporary directory.

### 3. Window Detection (`find-window-id.swift`)

A Swift helper that finds Shipyard's window ID by bundle identifier (`com.shipyard.app`) using `CGWindowListCopyWindowInfo`. This window ID is passed to `screencapture -l <id>` to capture the window without shadows or other desktop elements.

### 4. Main Capture Script (`capture.sh`)

The orchestration script:

1. **Launches Shipyard** with `HOME=<isolated-dir>` to isolate app data
2. **Waits for the window** to appear (up to 30 seconds)
3. **Positions and resizes** the window to 1440×900 logical points
4. **Navigates through the UI** using AppleScript/System Events:
   - Dashboard view
   - Repository detail with commit history
   - Working tree with file changes
   - Branch list
   - Settings/group configuration
5. **Captures each screenshot** with `screencapture -l <window-id> -o` (no shadow)
6. **Records a 10-15 second demo** showing:
   - Switching between repositories
   - Viewing changes and history
   - Navigating the UI
7. **Converts the video to GIF** using ffmpeg with palettegen/paletteuse for optimization:
   - 15 fps
   - 960px wide
   - Target size: ~8 MB

### Navigation Strategy

The script uses a combination of:
- **Keyboard shortcuts**: Cmd+1 for dashboard, Cmd+, for settings
- **AppleScript UI automation**: Clicking buttons, menu items, and specific coordinates
- **System Events**: Sending keystrokes and mouse events

The five screenshots correspond to the README's TODO comment (lines 41-47):
1. `01-dashboard.png` - Dashboard view with multiple repository groups
2. `02-history.png` - Commit history with visual graph and file changes
3. `03-working-tree.png` - Working tree with staged/unstaged changes and diff viewer
4. `04-branches.png` - Branch list with tracking information
5. `05-settings.png` - Repository settings showing group configuration

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
- Save everything to `docs/screenshots/` and `docs/`

### Dry Run

Preview what would be captured without actually taking screenshots:

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

The script leaves the generated files in:
- `docs/screenshots/*.png` - 5 screenshot images
- `docs/demo.gif` - Demo animation

Review these files, then:
1. Commit them to the repository
2. Push the branch
3. Open a pull request

The main README is already updated with relative paths to these images.

## Troubleshooting

### "Application with bundle identifier 'com.shipyard.app' is not running"

- Check that Shipyard is installed at `/Applications/Shipyard.app`
- Verify the app launches successfully
- The script may have failed to wait long enough - increase `max_wait` in `launch_app()`

### "Could not get window list" or "No visible window found"

- Grant **Screen Recording** permission to Terminal
- Try running `swift find-window-id.swift com.shipyard.app` manually
- The app window must be visible (not minimized or hidden)

### "Operation not permitted" when using System Events

- Grant **Accessibility** permission to Terminal
- Go to System Settings → Privacy & Security → Accessibility
- Add Terminal and restart it

### Screenshots are blank or wrong size

- Ensure the window is fully visible and not obstructed
- Check that window positioning succeeded before capture
- Try increasing sleep delays if the UI hasn't settled

### GIF size exceeds 8 MB

- Reduce frame rate (currently 15 fps) in the ffmpeg command
- Reduce the width (currently 960px)
- Shorten the recording duration
- Increase `bayer_scale` for more aggressive dithering

### Navigation fails or wrong UI appears

- The AppleScript navigation is fragile and depends on UI structure
- Adjust click coordinates or selectors in `navigate_to()` and `click_at()`
- Add more `sleep` delays between actions
- Consider using accessibility identifiers if the Vue components expose them

## Refreshing After UI Changes

After making changes to Shipyard's UI that affect the screenshots:

1. Install the new version: `bun run tauri build` and copy to `/Applications/`
2. Run the capture script: `bash scripts/capture/capture.sh`
3. Review the new screenshots and demo
4. Commit and push the updated images

The demo data generation is deterministic (uses fixed fake author names and deterministic random seeds), so repeated runs should produce similar repository content.

## Script Architecture

```
scripts/capture/
├── capture.sh                 # Main orchestration script
├── generate-demo-data.sh      # Creates demo Git repositories
├── seed-app-data.sh           # Populates Shipyard's settings.json
├── find-window-id.swift       # Finds window ID by bundle identifier
└── README.md                  # This file
```

All scripts:
- Use `set -euo pipefail` for safety
- Provide colored progress output
- Clean up temporary files on exit
- Print clear errors with context

## Assumptions and Limitations

### Verified

- App data directory location: `~/Library/Application Support/com.shipyard.app/`
- Settings file format: JSON matching `AppData` struct in `src-tauri/src/models.rs`
- Bundle identifier: `com.shipyard.app` (from `src-tauri/tauri.conf.json`)
- Window sizing works via AppleScript System Events

### Assumptions (Unverified Without Running macOS)

- UI navigation coordinates and element selectors may need adjustment
- Keyboard shortcuts (Cmd+1, Cmd+,) may not exist or may differ
- The menu structure for accessing History view
- Exact timing for UI settling after navigation
- Screenshot composition (whether all desired elements are visible in the frame)

### Limitations

- **macOS only** - These scripts cannot run on Linux or Windows
- **Requires installed app** - Cannot run against `bun run tauri dev`
- **Fragile UI automation** - AppleScript click coordinates break if layout changes
- **No headless mode** - The app must be visible and in the foreground
- **Timing-dependent** - Fixed `sleep` delays may be insufficient on slower machines

If the app is not behaving as expected, inspect:
- The Vue router in `src/router.ts`
- Keyboard shortcuts in `src/App.vue` (lines 160-175)
- Tab navigation in `src/composables/useTabs.ts`
- UI component structure in `src/views/` and `src/components/`
