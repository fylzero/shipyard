<p align="center">
  <img src="app-icon.png" alt="Shipyard" width="160" height="160">
</p>

<h1 align="center">Shipyard</h1>

<p align="center">
  A fast, native-feeling, open source Git client for macOS.
</p>

<p align="center">
  <a href="https://github.com/fylzero/shipyard/releases/latest/download/Shipyard-macos-arm64.dmg"><img src="docs/download-macos.svg" alt="Download Shipyard for macOS" height="56"></a>
  <a href="https://buymeacoffee.com/fylzero1"><img src="docs/buy-me-a-coffee.svg" alt="Buy me a coffee" height="56"></a>
</p>

## What is Shipyard?

Shipyard is a local-first Git client built with Tauri, Vue, and Rust. It talks directly to the `git` binary already on your machine, so it uses your existing SSH agent, credential helper, and user identity—no separate login required.

Shipyard is designed for developers who manage multiple repositories and want a native macOS experience with powerful Git features without sacrificing speed or control.

## Features

- **Repository Groups**: Organize multiple repositories into named groups with bulk operations (pull all, checkout all to a branch)
- **Working Tree Management**: Stage, unstage, commit, amend, and discard changes with a clear file tree view
- **Branch Operations**: Create, checkout, delete, merge, rename, and track branches with visual indicators for merged/unmerged status
- **Remote Management**: Add, edit, and remove remotes; fetch, pull, and push with conflict detection
- **Commit History**: Visual commit graph with searchable history, file changes, and diffs
- **Stash Support**: Save, apply, pop, and drop stashes with individual stash inspection
- **Tag Management**: Create, delete, and view both lightweight and annotated tags
- **File History & Blame**: Track changes to individual files over time with line-by-line authorship
- **Diff Viewer**: Inline or split diff views with syntax highlighting and customizable fonts
- **Cherry-pick & Revert**: Apply or undo commits across branches
- **Integrated Terminal**: Open terminals in repository directories without leaving the app
- **Smart Refresh**: Configurable auto-refresh with active hours to avoid unnecessary Git operations
- **Editor Integration**: Open files directly in Cursor, VS Code, Zed, Sublime Text, PhpStorm, or your system default
- **Notifications**: Background change notifications when repositories update
- **Command History**: Inspect every Git command Shipyard runs under the hood
- **Pull Request Helpers**: Generate GitHub/GitLab pull request URLs for branches

<!-- TODO: Add screenshots or demo GIF showing the main interface
Recommended screenshots to capture:
1. Dashboard view with multiple repository groups
2. Commit history with visual graph and file changes
3. Working tree with staged/unstaged changes and diff viewer
4. Branch list with tracking information
5. Repository settings showing group configuration
-->

## Download and Install

### Requirements

- **macOS**: Apple Silicon (ARM64) required
- **Git**: Must be installed and available in your PATH

### Installation

1. Download the latest release: [Shipyard-macos-arm64.dmg](https://github.com/fylzero/shipyard/releases/latest/download/Shipyard-macos-arm64.dmg)
2. Open the DMG and drag Shipyard to your Applications folder
3. Launch Shipyard from Applications

#### Opening an unsigned macOS app

If you see a warning that Shipyard cannot be opened because it is from an unidentified developer:

1. Right-click (or Control-click) the Shipyard app in Finder
2. Select **Open** from the context menu
3. Click **Open** in the confirmation dialog

This is only required the first time you launch a locally built or unsigned version. Signed releases from GitHub do not require this step.

**Note**: Current releases are built for **Apple Silicon (ARM64) Macs only**. Intel Macs are not currently supported.

## Development

Want to build Shipyard from source or contribute? You'll need **Bun**, **Rust**, **Xcode Command Line Tools**, and **Git**.

### Prerequisites

#### 1. Xcode Command Line Tools

These provide the C/C++ compiler Rust uses on macOS.

```bash
xcode-select --install
```

If already installed, confirm with:

```bash
xcode-select -p
```

You should see `/Library/Developer/CommandLineTools` or an Xcode path.

#### 2. Bun

Install with the official script from [bun.sh](https://bun.sh/):

```bash
curl -fsSL https://bun.sh/install | bash
bun --version
```

#### 3. Rust

Tauri compiles the native app with `cargo`. Install the official toolchain with [rustup](https://www.rust-lang.org/tools/install):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Accept the defaults, then **open a new terminal**. Confirm:

```bash
cargo --version
rustc --version
```

#### 4. Git

```bash
git --version
```

Homebrew Git and `/usr/bin/git` both work. Shipyard shells out to whatever `git` is on your PATH.

### Building and Running

#### Clone the repository

```bash
git clone https://github.com/fylzero/shipyard.git
cd shipyard
```

#### Install dependencies and start the app

```bash
bun install
bun run tauri dev
```

This command:

1. Starts Vite on `http://localhost:1430` for the Vue UI
2. Compiles the Rust/Tauri shell
3. Opens the native Shipyard window

The **first** `tauri dev` (or `tauri build`) downloads crates and compiles from scratch, which often takes several minutes. Later runs are much faster.

#### Export a native Mac app

```bash
bun run tauri build
```

When the build finishes:

| Artifact | Path |
| --- | --- |
| App bundle | `src-tauri/target/release/bundle/macos/Shipyard.app` |
| Disk image | `src-tauri/target/release/bundle/dmg/Shipyard_1.15.0_aarch64.dmg` |

To run it locally, open the `.app` (or drag it to `/Applications`). macOS Gatekeeper may warn that an unsigned local build is unidentified: right-click the app, choose **Open**, then confirm.

### Publishing Signed Releases

See [macOS release setup](docs/macos-releases.md) for Apple enrollment, repository secrets, and the signing and notarization checks required by the Release workflow.

### Troubleshooting

**`failed to run 'cargo metadata' ... No such file or directory (os error 2)`**  
Rust is not installed. Run step 3 above, open a new terminal, and confirm `command -v cargo` prints `/Users/<you>/.cargo/bin/cargo`. If rustup did not update `~/.zshrc`, add `. "$HOME/.cargo/env"` there once.

**`xcrun: error: invalid active developer path`**  
Xcode Command Line Tools are missing or stale. Run `xcode-select --install` (or `sudo xcode-select --reset` after installing Xcode).

**Vite starts but no native window**  
Leave the first Rust compile running. If it fails, the error is in the same terminal as `bun run tauri dev`.

**Frontend only (browser, no native APIs)**  
`bun run dev` serves the Vue app at `http://localhost:1430`. Git, dialogs, and other Tauri APIs need `bun run tauri dev`.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style guidelines, and how to submit pull requests.

## License

MIT
