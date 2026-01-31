# ManaHg

<div align="center">
  <img src="assets/ManaHg.png" alt="ManaHg Logo" width="200">
</div>

A fast, native GUI tool for managing multiple Mercurial (Hg) repositories, written in Rust with FLTK.

## Features

- **Multi-Repo Dashboard**: Monitor path, current branch, revision, modification status, phase, and last operation status for many repositories at once.
- **Bulk Operations**: 
  - **Pull**: Pull all branches or just the current branch.
  - **Update**: Update to the latest revision or a specific tag.
  - **Switch Branch**: Switch branches across multiple selected repositories.
  - **Commit**: Perform quick commits on selected repositories.
  - **Refresh**: Fast, parallel status checking.
- **Integration**:
  - Open repositories directly in **TortoiseHg**.
  - Copy repository paths to clipboard.
- **User Interface**: 
  - Context menu for quick access to actions.
  - Sortable columns.
  - Multiple themes (Greybird, Dark, Metro, Blue, HighContrast).
- **Portable**: Compiles to a single standalone executable.

## Prerequisites

- **Rust** (for building): [Install Rust](https://rustup.rs/)
- **Mercurial**: `hg` command must be in your system PATH.
- **TortoiseHg** (Optional): Required for "Open in TortoiseHg" feature.

## Building

```bash
# Debug run
cargo run

# Release build (Optimized)
cargo build --release
```

The compiled binary will be in `target/release/ManaHg.exe`.

## Usage

### Managing Repositories
- **Add**: Use `File > Search for repos...` (Ctrl++) to scan a folder hierarchy for `.hg` repositories.
- **Remove**: Select repositories and press `Del` or use `File > Remove` to remove them from the list (does not delete files).

### Operations
Select one or more repositories in the list to perform actions:
- **Right-Click**: Opens the context menu with all available actions.
- **Menu Bar**: Access actions via the `Action` menu.
- **Double-Click**: Opens the repository in TortoiseHg.

### Available Actions
- **Pull**: Fetch changes from the remote server.
- **Update to Latest**: Update to the tip of the current branch.
- **Update to Tag...**: Select a tag from the collective list of tags in selected repos.
- **Switch Branch...**: Switch to a common branch found in the selected repos.
- **Commit...**: Commit changes with a message.
- **Copy**: Copy the path of selected repositories to clipboard.

## Configuration

The application saves your repository list and preferences in `configuration.json` in the same directory as the executable.

## License

MIT
