<p align="center">
  <img src="app-icon.png" alt="Shipyard" width="160" height="160">
</p>

<h1 align="center">Shipyard</h1>

<p align="center">
  A local-first Git client for macOS. Shipyard talks to the <code>git</code> binary already on your machine, so it uses the same SSH agent, credential helper, and user identity you use in Terminal. There is no in-app login.
</p>

<p align="center">
  <a href="https://github.com/fylzero/shipyard/releases/latest/download/Shipyard-macos-arm64.dmg"><img src="docs/download-macos.svg" alt="Download Shipyard for macOS" height="56"></a>
  <a href="https://buymeacoffee.com/fylzero1"><img src="docs/buy-me-a-coffee.svg" alt="Buy me a coffee" height="56"></a>
</p>

## First-time setup

Shipyard is a [Tauri](https://v2.tauri.app/) app: a Vue frontend plus a Rust native shell. You need **Bun**, **Rust**, **Xcode Command Line Tools**, and **Git** before `bun run tauri dev` will work.

### 1. Clone the repo

```bash
git clone https://github.com/fylzero/shipyard.git
cd shipyard
```

### 2. Xcode Command Line Tools

These provide the C/C++ compiler Rust uses on macOS.

```bash
xcode-select --install
```

If that says they are already installed, you are fine. Confirm with:

```bash
xcode-select -p
```

You should see `/Library/Developer/CommandLineTools` or an Xcode path.

### 3. Bun

Install with the official script from [bun.sh](https://bun.sh/):

```bash
curl -fsSL https://bun.sh/install | bash
bun --version
```

### 4. Rust (this is the usual missing piece)

Tauri compiles the native app with `cargo`. If Rust is missing you will get:

```text
failed to run 'cargo metadata' ... No such file or directory (os error 2)
```

Install the official toolchain with [rustup](https://www.rust-lang.org/tools/install):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Accept the defaults, then **open a new terminal**. rustup puts `cargo` on your PATH via `~/.zshrc`. Confirm:

```bash
cargo --version
rustc --version
```

You do not need to `source` anything for `bun run tauri dev`. That script already prepends `$HOME/.cargo/bin` to `PATH`.

### 5. Git

```bash
git --version
```

Homebrew Git and `/usr/bin/git` are both fine. Shipyard shells out to whatever `git` is on your PATH.

### 6. Install JS dependencies and start the app

```bash
bun install
bun run tauri dev
```

That command:

1. Starts Vite on `http://localhost:1420` for the Vue UI
2. Compiles the Rust/Tauri shell
3. Opens the native Shipyard window

The **first** `tauri dev` (or `tauri build`) downloads crates and compiles from scratch. That often takes several minutes. Later runs are much faster.

## Export a native Mac app

```bash
bun run tauri build
```

When the build finishes:

| Artifact | Path |
| --- | --- |
| App bundle | `src-tauri/target/release/bundle/macos/Shipyard.app` |
| Disk image | `src-tauri/target/release/bundle/dmg/Shipyard_1.1.1_aarch64.dmg` |

To run it locally, open the `.app` (or drag it to `/Applications`). macOS Gatekeeper may warn that an unsigned local build is unidentified: right-click the app, choose **Open**, then confirm.

## Publishing signed releases

See [macOS release setup](docs/macos-releases.md) for Apple enrollment, repository
secrets, and the signing and notarization checks required by the Release workflow.

## Troubleshooting

**`failed to run 'cargo metadata' ... No such file or directory (os error 2)`**  
Rust is not installed. Run step 4, open a new terminal, and confirm `command -v cargo` prints `/Users/<you>/.cargo/bin/cargo`. If rustup did not update `~/.zshrc`, add `. "$HOME/.cargo/env"` there once.

**`xcrun: error: invalid active developer path`**  
Xcode Command Line Tools are missing or stale. Run `xcode-select --install` (or `sudo xcode-select --reset` after installing Xcode).

**Vite starts but no native window**  
Leave the first Rust compile running. If it fails, the error is in the same terminal as `bun run tauri dev`.

**Frontend only (browser, no native APIs)**  
`bun run dev` serves the Vue app at `http://localhost:1420`. Git, dialogs, and other Tauri APIs need `bun run tauri dev`.
