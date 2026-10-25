# Contributing to Shipyard

Thanks for your interest in contributing to Shipyard! This document provides guidelines for developing and contributing to the project.

## Development Setup

Shipyard is a [Tauri](https://v2.tauri.app/) app with a Vue frontend and Rust backend. You'll need the following tools installed:

- **Bun** - JavaScript runtime and package manager ([bun.sh](https://bun.sh/))
- **Rust** - via [rustup](https://www.rust-lang.org/tools/install)
- **Xcode Command Line Tools** - provides the C/C++ compiler for macOS
- **Git** - Shipyard shells out to your system's `git` binary

### Getting Started

1. Clone the repository:
   ```bash
   git clone https://github.com/fylzero/shipyard.git
   cd shipyard
   ```

2. Install Xcode Command Line Tools (if not already installed):
   ```bash
   xcode-select --install
   ```

3. Install Bun:
   ```bash
   curl -fsSL https://bun.sh/install | bash
   ```

4. Install Rust:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
   After installation, open a new terminal or run `. "$HOME/.cargo/env"`

5. Install JavaScript dependencies:
   ```bash
   bun install
   ```

6. Start the development server:
   ```bash
   bun run tauri dev
   ```

The first build will take several minutes as it downloads and compiles Rust dependencies. Subsequent builds are much faster.

## Code Style

### Frontend (Vue/TypeScript)
- Follow the existing TypeScript and Vue component patterns
- Use Composition API with `<script setup>` syntax
- Type your components and functions
- Keep components focused and reasonably sized

### Backend (Rust)
- Follow standard Rust conventions and idioms
- Use `cargo fmt` to format code
- Address `cargo clippy` warnings when practical

## Running Checks

Before submitting a pull request, run the following checks:

### TypeScript Type Check
```bash
bun run build
```
This runs `vue-tsc --noEmit` and `vite build` to catch type errors.

### Rust Check
```bash
cd src-tauri
cargo check
cargo clippy
```

### Build a Release Bundle (Optional)
```bash
bun run tauri build
```
The built app will be at `src-tauri/target/release/bundle/macos/Shipyard.app`

## Opening Issues

- **Bug reports**: Describe what you expected to happen, what actually happened, and steps to reproduce. Include the macOS version and Shipyard version if relevant.
- **Feature requests**: Explain the use case and why the feature would be valuable.

Use the issue templates when available to ensure you provide all the necessary information.

## Submitting Pull Requests

1. Create a feature branch from `main`
2. Make your changes with clear, focused commits
3. Test your changes thoroughly
4. Push your branch and open a pull request
5. Fill out the pull request template
6. Be responsive to code review feedback

### PR Guidelines
- Keep pull requests focused on a single feature or fix
- Write clear commit messages
- Update documentation if you change behavior
- Don't modify version numbers, release workflows, or application secrets

## Questions?

If you have questions about contributing, feel free to open an issue for discussion before starting work on a significant change.
