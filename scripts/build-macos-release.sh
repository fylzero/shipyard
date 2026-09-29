#!/usr/bin/env bash
set -euo pipefail

# tauri-action invokes this with: build --target aarch64-apple-darwin.
# APPLE_SIGNING_IDENTITY overrides the local ad-hoc identity in tauri.conf.json.
# Tauri imports the certificate, signs, notarizes with notarytool, and staples
# the app before creating the DMG and signed updater archive.
bun run tauri "$@"

bundle="src-tauri/target/aarch64-apple-darwin/release/bundle"

verify_app() {
  local app="$1"
  local details
  codesign --verify --deep --strict --verbose=2 "$app"
  details="$(codesign --display --verbose=4 "$app" 2>&1)"
  if ! grep -Fq "Authority=$APPLE_SIGNING_IDENTITY" <<< "$details"; then
    echo "::error::Unexpected signing identity in $app"
    return 1
  fi
  if ! grep -Fxq "TeamIdentifier=$APPLE_TEAM_ID" <<< "$details"; then
    echo "::error::Unexpected signing team in $app"
    return 1
  fi
  if ! grep -Eq 'flags=.*\(.*runtime.*\)' <<< "$details"; then
    echo "::error::Hardened Runtime is missing from $app"
    return 1
  fi
  xcrun stapler validate "$app"
  spctl --assess --type execute --verbose=2 "$app"
}

verify_app "$bundle/macos/Shipyard.app"

# Verify the actual distribution contents, including the stapled ticket.
scratch="$(mktemp -d)"
mounted=0
cleanup() {
  if [[ "$mounted" == 1 ]]; then
    hdiutil detach "$scratch/mount" || true
  fi
  rm -rf "$scratch"
}
trap cleanup EXIT
mkdir "$scratch/updater" "$scratch/mount"
tar -xzf "$bundle/macos/Shipyard.app.tar.gz" -C "$scratch/updater"
verify_app "$scratch/updater/Shipyard.app"

shopt -s nullglob
dmgs=("$bundle"/dmg/*.dmg)
if [[ "${#dmgs[@]}" != 1 ]]; then
  echo "::error::Expected exactly one release DMG"
  exit 1
fi
hdiutil attach "${dmgs[0]}" -readonly -nobrowse -mountpoint "$scratch/mount"
mounted=1
verify_app "$scratch/mount/Shipyard.app"
hdiutil detach "$scratch/mount"
mounted=0
