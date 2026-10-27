# Contributing to Shipyard

Thanks for your interest in contributing to Shipyard! This document provides guidelines for developing and contributing to the project.

## Development Setup

Shipyard is a [Tauri](https://v2.tauri.app/) app with a Vue frontend and Rust backend. Follow the [Development section of the README](README.md#development) to install the prerequisites (Bun, Rust, Xcode Command Line Tools, and Git) and run the app with `bun run tauri dev`.

## Code Style

### Frontend (Vue/TypeScript)
- Follow the existing TypeScript and Vue component patterns
- Use Composition API with `<script setup>` syntax
- Type your components and functions
- Keep components focused and reasonably sized

### Backend (Rust)
- Follow standard Rust conventions and idioms
- Run `cargo fmt` to format code
- Fix all `cargo clippy` warnings; CI treats them as errors

## Running Checks

Before submitting a pull request, run the following checks. CI runs the same checks on every pull request.

### TypeScript Type Check
```bash
bun run build
```
This runs `vue-tsc --noEmit` and `vite build` to catch type errors.

### Rust Check
```bash
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

### Build a Release Bundle (Optional)
```bash
bun run tauri build
```
The built app will be at `src-tauri/target/release/bundle/macos/Shipyard.app`

## Opening Issues

- **Bug reports**: Describe what you expected to happen, what actually happened, and steps to reproduce. Include your operating system (macOS version, Windows version, or Linux distro and desktop) and Shipyard version if relevant.
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
