# Implemented Zake v1 Foundation

This document records what has been implemented so far in Zake and how the pieces fit together.

## Summary

Zake is currently a Rust CLI/TUI application for managing a Markdown notebook backed by Git. The implemented version covers the first usable foundation: notebook initialization, note metadata management, lazy indexing, terminal navigation, Git status/actions, ripgrep search, and diagnostics.

The implementation follows the planned constraint that Zake manages notes but does not become an internal note body reader or editor.

## User-Facing Features

Notebook management:

- `zake init [path]` creates `.zake/config.toml` and `.zake/cache/`.
- `zake init` initializes Git when the notebook root is not already a Git repo.
- Running `zake` without a subcommand discovers a notebook from the current directory upward.
- If no notebook is found during TUI startup, Zake prompts to initialize the current directory.
- `zake doctor [path]` validates config, Git availability, ripgrep availability, index health, parse errors, and broken links.

Note management:

- `zake new "<title>" [path]` creates a Markdown file with YAML frontmatter and a heading stub.
- `zake new` accepts `--type`, repeated `--tag`, and repeated `--link` flags for creating notes with metadata.
- CLI commands support listing, showing, renaming, moving, deleting, opening, and updating note metadata by exact title or path.
- TUI commands support creating, renaming, moving, deleting, and updating note metadata.
- Metadata commands update only frontmatter fields: tags, type, and links.
- `:open` launches the selected note in `$EDITOR` and refreshes state after the editor exits.

Navigation and indexing:

- Startup builds a lazy in-memory index from Markdown files.
- The index maps note titles, tags, types, outgoing links, backlinks, and broken links.
- Inline link extraction supports wiki links like `[[Title]]` and Markdown links like `[label](target)`.
- Fuzzy matching is implemented for note title/path search in the index layer.

Git:

- Git integration shells out to the installed `git` binary.
- Status uses `git status --porcelain=v1 -z`.
- TUI commands support `:stage`, `:unstage`, `:stage-all`, and `:commit <message>`.
- Recent history uses `git log --oneline --decorate`.
- `unstage` handles both normal repositories and unborn repositories before the first commit.

Search:

- `zake search "<query>" [path]` runs ripgrep and prints `path:line:text` hits.
- TUI `:search <query>` shows search hits in the search pane.
- Search results are navigable references, not an internal note reading mode.

## Architecture

The crate is split by responsibility:

- `src/main.rs`: command dispatch and TUI startup.
- `src/cli.rs`: Clap command definitions.
- `src/notebook.rs`: notebook config, init, discovery, loading, and validation.
- `src/note.rs`: note metadata, Markdown frontmatter parsing, note lifecycle operations, and inline link extraction.
- `src/index.rs`: in-memory note index, tag/type/title maps, fuzzy matching, backlinks, and broken-link tracking.
- `src/git.rs`: shell-backed Git porcelain parsing and Git actions.
- `src/search.rs`: ripgrep search wrapper and result parsing.
- `src/app.rs`: shared TUI application state and command execution.
- `src/tui.rs`: Ratatui/Crossterm rendering and keyboard event loop.
- `src/lib.rs`: module exports for tests and binary use.

Key dependency choices:

- `ratatui` and `crossterm` for the TUI.
- `clap` for CLI parsing.
- `serde`, `serde_yaml`, and `toml` for note and notebook metadata.
- `ignore` for Git-ignore-aware note walking.
- `fuzzy-matcher` for lightweight fuzzy note matching.
- Shell `git` and shell `rg` for external workflows instead of embedding heavier engines.

## Data Model

Notebook layout:

```text
notebook-root/
  .zake/
    config.toml
    cache/
  note-files.md
```

Current `.zake/config.toml` shape:

```toml
version = 1
notes_dir = "."
```

Current note frontmatter:

```yaml
title: My Note
type: note
tags: []
links: []
created_at: 2026-06-05T12:00:00Z
updated_at: 2026-06-05T12:00:00Z
```

## Tests

Implemented test coverage includes:

- Note slug generation.
- YAML/frontmatter note lifecycle through create, rename, move, delete, and index rebuild.
- Markdown and wiki link extraction.
- Tag/type indexing and broken-link detection.
- Git porcelain `-z` status parsing.
- Real temporary Git repo workflow for stage, unstage, commit, status, and history.
- Ripgrep line parsing.

Run all tests with:

```sh
cargo test
```

## Current Limits

- The TUI is a functional first shell, not a polished final UX.
- There is no persistent index or database.
- Fuzzy matching exists in the index layer but there is not yet a dedicated interactive fuzzy picker pane.
- Deleting a note currently deletes immediately from the command; a richer confirmation flow should be added before broad use.
- Metadata command parsing is whitespace-based, so quoted multi-word tags/links are not yet supported.
- Git commit prompts are single-line command entries only.
- Search depends on `rg` being installed.
- Git features depend on the installed `git` binary.
- There is no package/install workflow yet beyond `cargo build` / `cargo run`.

## Verification Performed

The current implementation has been verified with:

```sh
cargo fmt
cargo test
```

It has also been smoke-tested manually by initializing a temporary notebook, creating a note, running `doctor`, and searching for content with ripgrep.
