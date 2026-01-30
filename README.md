# ManaHg

A fast, native GUI tool for managing multiple Mercurial (Hg) repositories, written in Rust with FLTK.

## Features

- **Dashboard View**: Monitor path, branch, revision, modification status, and phase for multiple repos.
- **Bulk Actions**: 
  - Pull (All Branches / Current Branch)
  - Update to Latest
  - Refresh status
- **Commit**: Simple commit interface for selected repositories.
- **Concurrency**: Fast directory scanning and parallel operations using threading.
- **Customization**: 
  - Sortable columns.
  - Multiple themes (Greybird, Dark, Metro, etc.).
- **Portable**: Compiles to a single standalone executable.

## Prerequisites

- **Rust** (for building): [Install Rust](https://rustup.rs/)
- **Mercurial**: `hg` command must be in your system PATH.

## Building

```bash
# Debug run
cargo run

# Release build (Optimized)
cargo build --release
```

The compiled binary will be in `target/release/ManaHg.exe`.

## Usage

1. **Add**: Use `File > Add Repository` to select a folder containing `.hg`.
2. **Refresh**: Press `F5` or use the Refresh button.
3. **Sort**: Click column headers to sort by Path, Branch, Status, etc.
4. **Theme**: Go to `File > Preferences` to change the visual theme.

## Configuration

The application saves your repository list and preferences in `configuration.json` in the same directory as the executable.
