// Prints the GitHub release body for a version: its notes from src/changelog.ts
// (the same list the in-app Changelog shows), then the download instructions.
//
//   bun scripts/release-notes.ts            # version from src-tauri/tauri.conf.json
//   bun scripts/release-notes.ts 1.2.0      # a specific version
//   bun scripts/release-notes.ts --since v1.1.0
//                                           # also every version after 1.1.0, for
//                                           # versions bumped but never released
//
// Exits with an error when the changelog has no entry for the version, so a
// release can't go out without its notes.

import { readFileSync } from "node:fs";
import { CHANGELOG } from "../src/changelog";

const DOWNLOADS = [
  "macOS: download Shipyard-macos-arm64.dmg for Apple Silicon Macs, or run `brew install fylzero/tap/shipyard`.",
  "Linux: download Shipyard-linux-x64.AppImage, .deb, or .rpm for Intel and AMD PCs, or the Shipyard-linux-arm64 files for ARM.",
  "Windows: download Shipyard-windows-x64.exe for Intel and AMD PCs, or Shipyard-windows-arm64.exe for ARM.",
  "The Windows installer isn't code-signed yet, so SmartScreen may warn: click More info, then Run anyway.",
];

function configVersion(): string {
  const url = new URL("../src-tauri/tauri.conf.json", import.meta.url);
  return JSON.parse(readFileSync(url, "utf8")).version;
}

const args = process.argv.slice(2);
const sinceIndex = args.indexOf("--since");
const since = sinceIndex >= 0 ? args.splice(sinceIndex, 2)[1]?.replace(/^v/, "") : undefined;
const version = (args[0] ?? configVersion()).replace(/^v/, "");

function compare(a: string, b: string): number {
  const pa = a.split(".").map(Number);
  const pb = b.split(".").map(Number);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d !== 0) return d;
  }
  return 0;
}

const release = CHANGELOG.find((r) => r.version === version);

if (!release || release.notes.length === 0) {
  console.error(`src/changelog.ts has no notes for version ${version}. Add them before releasing.`);
  process.exit(1);
}

// This version plus any between it and the last published release, newest first.
const releases = CHANGELOG.filter(
  (r) =>
    r.version === version ||
    (since !== undefined && compare(r.version, since) > 0 && compare(r.version, version) < 0),
).sort((a, b) => compare(b.version, a.version));

const whatsNew =
  releases.length === 1
    ? release.notes.map((note) => `- ${note}`)
    : releases.flatMap((r, i) => [
        ...(i > 0 ? [""] : []),
        `### ${r.version}`,
        "",
        ...r.notes.map((note) => `- ${note}`),
      ]);

const body = [
  "## What's new",
  "",
  ...whatsNew,
  "",
  "## Download",
  "",
  ...DOWNLOADS.map((line) => `- ${line}`),
];

console.log(body.join("\n"));
