# Zake

Zake is a fast, Git-backed terminal UI for managing Markdown notes. It helps you create and organize notebooks, tags, types, links, search results, and Git changes without becoming an in-app note reader or editor.

The core idea is simple: your notes stay as portable Markdown files, your history stays in Git, and Zake gives you a lightweight control surface for managing the system around them.

## Current Status

Zake has a working v1 foundation:

- Notebook initialization with `.zake/config.toml`
- Markdown note stubs with YAML frontmatter
- In-memory note index for titles, tags, types, links, backlinks, and broken links
- Ratatui-based TUI with notes, fuzzy search, metadata, Git, search/history, and command panes
- Shell-backed Git status, stage, unstage, stage-all, commit, and log history
- Explicit Git convenience commands while leaving the repository user-owned
- Ripgrep-backed search results
- External editor handoff through `$EDITOR`
- CLI helpers for initialization, diagnostics, note creation, graph health, rituals, and search

Zake intentionally does not render full note bodies as a reading view and does not provide an internal body editor.

## Install And Run

For final users, see [docs/INSTALL.md](docs/INSTALL.md).

Quick source install:

```sh
cargo install --path .
```

After installation, run Zake directly:

```sh
zake --help
zake init ~/notes
cd ~/notes
zake
```

For development, build from source:

```sh
cargo build
```

Run tests:

```sh
cargo test
```

Start the TUI from inside a notebook:

```sh
zake
```

If the current directory is not a Zake notebook, the TUI startup path will ask whether to initialize one there.

## CLI

Initialize a notebook:

```sh
zake init [path]
```

Check notebook health:

```sh
zake doctor [path]
```

Create a note stub:

```sh
zake new "My Note Title" --type idea --tag rust --tag cli --link "Another Note" [path]
```

List notes, optionally filtered by tag or type:

```sh
zake list [path]
zake list --tag rust [path]
zake list --type idea [path]
```

Inspect a note by exact title or path:

```sh
zake show "My Note Title" [path]
```

Update note metadata from the CLI:

```sh
zake set "My Note Title" --type reference --tag rust --tag docs --link "Related Note" [path]
zake set "My Note Title" --clear-tags --clear-links [path]
```

Manage note files from the CLI:

```sh
zake rename "My Note Title" "Better Title" [path]
zake rename "My Note Title" "Better Title" --update-links [path]
zake move "Better Title" archive [path]
zake delete "Better Title" [path]
zake delete "Better Title" --yes [path]
zake open "Better Title" [path]
```

Inspect graph health:

```sh
zake links "My Note Title" [path]
zake backlinks "My Note Title" [path]
zake broken [path]
zake orphans [path]
```

Create ritual notes:

```sh
zake today [path]
zake week [path]
```

Search note files with ripgrep:

```sh
zake search "query" [path]
```

Manage notebook Git history:

```sh
zake status [path]
zake stage note.md [path]
zake unstage note.md [path]
zake stage-all [path]
zake commit "Update notes" [path]
zake history [path]
zake diff [note.md] --path [path]
zake snapshot "End of day notes" [path]
```

Zake does not hide Git. These commands are convenience wrappers for common
notebook maintenance, and you can still use normal `git` commands in the notebook
repo whenever you want.

## TUI Basics

The TUI is an interactive control surface made of movable panes. It keeps note
body editing in your configured editor, but makes note navigation, metadata,
links, search results, and Git history quick to operate from one terminal.

Navigation:

- `j` / `Down`: move selection down
- `k` / `Up`: move selection up
- `Tab`: cycle panes
- `Shift-Tab`: cycle panes backward
- `w`: cycle pane arrangements
- `m`: zoom or unzoom the focused pane
- `Enter`: act on the focused pane
- `/` / `f`: fuzzy-find notes
- `:`: open command prompt
- `?`: show command help
- `r`: refresh index and Git state
- `n`: start `new` command
- `q`: quit

Focused pane actions:

- Notes or metadata: `Enter` opens the selected note in `$EDITOR`.
- Git: `Enter` toggles the selected path between staged and unstaged.
- Search results: `Enter` opens the matching note in `$EDITOR`.
- History, diff, links, backlinks, broken links, and orphan reports are shown in
  the search/history pane and can be selected with normal navigation.

Fast actions:

- `l`: start a link command prefilled with the selected note title.
- `h`: show recent Git history.
- `d`: show the selected note's Git diff.
- `z`: snapshot immediately with a timestamped commit message.
- `s`: stage the selected Git path.
- `u`: unstage the selected Git path.
- `Ctrl-a`: stage all Git changes.

Commands:

- `:new <title>`
- `:rename <title>`
- `:rename <title> --update-links`
- `:move <folder>`
- `:delete`
- `:delete!`
- `:tag <tag>...`
- `:type <kind>`
- `:link <target>...`
- `:open`
- `:search <query>`
- `:links`
- `:backlinks`
- `:broken`
- `:orphans`
- `:stage`
- `:unstage`
- `:stage-all`
- `:commit <message>`
- `:status`
- `:history [limit]`
- `:diff [path]`
- `:snapshot <message>`
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
