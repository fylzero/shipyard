// Prints the GitHub release body for a version: its notes from src/changelog.ts
// (the same list the in-app Changelog shows), then the download instructions.
//
//   bun scripts/release-notes.ts            # version from src-tauri/tauri.conf.json
//   bun scripts/release-notes.ts 1.2.0      # a specific version
//
// Exits with an error when the changelog has no entry for the version, so a
// release can't go out without its notes.

import { readFileSync } from "node:fs";
import { CHANGELOG } from "../src/changelog";

const DOWNLOADS = [
  "macOS: download Shipyard-macos-arm64.dmg for Apple Silicon Macs.",
  "Linux: download Shipyard-linux-x64.AppImage, .deb, or .rpm for Intel and AMD PCs, or the Shipyard-linux-arm64 files for ARM.",
  "Windows: download Shipyard-windows-x64.exe for Intel and AMD PCs, or Shipyard-windows-arm64.exe for ARM.",
  "The Windows installer isn't code-signed yet, so SmartScreen may warn: click More info, then Run anyway.",
];

function configVersion(): string {
  const url = new URL("../src-tauri/tauri.conf.json", import.meta.url);
  return JSON.parse(readFileSync(url, "utf8")).version;
}

const version = (process.argv[2] ?? configVersion()).replace(/^v/, "");
const release = CHANGELOG.find((r) => r.version === version);

if (!release || release.notes.length === 0) {
  console.error(`src/changelog.ts has no notes for version ${version}. Add them before releasing.`);
  process.exit(1);
}

const body = [
  "## What's new",
  "",
  ...release.notes.map((note) => `- ${note}`),
  "",
  "## Download",
  "",
  ...DOWNLOADS.map((line) => `- ${line}`),
];

console.log(body.join("\n"));
