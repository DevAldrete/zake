# Zake

Zake is a fast, Git-backed terminal UI for managing Markdown notes. It helps you create and organize notebooks, tags, types, links, search results, and Git changes without becoming an in-app note reader or editor.

The core idea is simple: your notes stay as portable Markdown files, your history stays in Git, and Zake gives you a lightweight control surface for managing the system around them.

## Current Status

Zake has a working v1 foundation:

- Notebook initialization with `.zake/config.toml`
- Markdown note stubs with YAML frontmatter
- In-memory note index for titles, tags, types, links, backlinks, and broken links
- Ratatui-based TUI with notes, metadata, Git, search/history, and command panes
- Shell-backed Git status, stage, unstage, stage-all, commit, and log history
- Ripgrep-backed search results
- External editor handoff through `$EDITOR`
- CLI helpers for initialization, diagnostics, note creation, and search

Zake intentionally does not render full note bodies as a reading view and does not provide an internal body editor.

## Install And Run

Build from source:

```sh
cargo build
```

Run tests:

```sh
cargo test
```

Start the TUI from inside a notebook:

```sh
cargo run
```

If the current directory is not a Zake notebook, the TUI startup path will ask whether to initialize one there.

## CLI

Initialize a notebook:

```sh
cargo run -- init [path]
```

Check notebook health:

```sh
cargo run -- doctor [path]
```

Create a note stub:

```sh
cargo run -- new "My Note Title" --type idea --tag rust --tag cli --link "Another Note" [path]
```

List notes, optionally filtered by tag or type:

```sh
cargo run -- list [path]
cargo run -- list --tag rust [path]
cargo run -- list --type idea [path]
```

Inspect a note by exact title or path:

```sh
cargo run -- show "My Note Title" [path]
```

Update note metadata from the CLI:

```sh
cargo run -- set "My Note Title" --type reference --tag rust --tag docs --link "Related Note" [path]
cargo run -- set "My Note Title" --clear-tags --clear-links [path]
```

Manage note files from the CLI:

```sh
cargo run -- rename "My Note Title" "Better Title" [path]
cargo run -- move "Better Title" archive [path]
cargo run -- delete "Better Title" [path]
cargo run -- open "Better Title" [path]
```

Search note files with ripgrep:

```sh
cargo run -- search "query" [path]
```

## TUI Basics

Navigation:

- `j` / `Down`: move selection down
- `k` / `Up`: move selection up
- `Tab`: cycle panes
- `:`: open command prompt
- `?`: show command help
- `r`: refresh index and Git state
- `n`: start `new` command
- `q`: quit

Commands:

- `:new <title>`
- `:rename <title>`
- `:move <folder>`
- `:delete`
- `:tag <tag>...`
- `:type <kind>`
- `:link <target>...`
- `:open`
- `:search <query>`
- `:stage`
- `:unstage`
- `:stage-all`
- `:commit <message>`
- `:refresh`
- `:quit`

## Note Format

Notes are Markdown files with YAML frontmatter:

```md
---
title: My Note
type: note
tags:
- rust
- ideas
links:
- Another Note
created_at: 2026-06-05T12:00:00Z
updated_at: 2026-06-05T12:00:00Z
---

# My Note
```

Zake manages the metadata and file lifecycle. Body writing and reading are left to your editor and normal file tools.

## Documentation

See [docs/IMPLEMENTED.md](docs/IMPLEMENTED.md) for the implementation ledger, architecture notes, current feature coverage, tests, and known limits.
