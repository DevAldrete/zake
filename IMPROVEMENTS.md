# Zake Improvements

This document collects high-leverage improvements that fit Zake's current
philosophy: Markdown files stay portable, Git owns history, and Zake provides a
fast terminal control surface around the notebook.

The goal is not to turn Zake into a full note reader, editor, or database. The
best features should make navigation, metadata hygiene, linking, and Git-backed
maintenance easier while preserving ordinary Markdown workflows.

## Product Philosophy

Zake should behave like a notebook operations console:

- It manages note files without hiding them.
- It improves notebook structure without requiring a proprietary format.
- It accelerates common actions without replacing the user's editor.
- It makes Git-backed history visible and low-friction.
- It surfaces graph health, broken links, and stale metadata before the notebook
  quietly decays.

This suggests a useful boundary:

- In scope: creating notes, finding notes, opening notes, changing metadata,
  managing links, checking health, staging and committing changes.
- Out of scope: rich body editing, rendered reading mode, sync services,
  hosted accounts, or a private database that becomes the source of truth.

## 1. Interactive Fuzzy Picker

### Why

The index layer already supports fuzzy note matching, but the TUI does not yet
expose it as a first-class workflow. A fuzzy picker would make Zake feel much
faster without changing its identity.

This is likely the highest-value next feature because it improves the core loop:
find a note, inspect its metadata, open it in `$EDITOR`, stage changes, commit.

### Proposed Behavior

- Press `/` or `f` in normal mode to enter note search mode.
- Typing filters notes by title and path using the existing fuzzy matcher.
- Results appear in the notes pane or a temporary picker pane.
- `Up` / `Down` or `j` / `k` moves through matches.
- `Enter` selects the highlighted note and returns to normal mode.
- Optional follow-up: `Ctrl-o` or `o` opens the highlighted note in `$EDITOR`.
- `Esc` cancels and restores the previous selection.

### Implementation Notes

- Reuse `NoteIndex::fuzzy_notes` from `src/index.rs`.
- Add a TUI mode such as `Mode::Find { query, previous_selection }`.
- Keep the picker as a navigation aid, not a content preview.
- Consider showing title, type, tags, and relative path for each match.

### Success Criteria

- A notebook with many notes can be navigated without repeated `j` / `k`.
- The feature works entirely from the keyboard.
- It does not render or edit note bodies.

## 2. Safer Destructive Actions

### Why

Deleting a note currently happens immediately from the command path. That is
acceptable for an early version, but it is risky once users trust Zake with real
notebooks.

The project is Git-backed, but Git should be a recovery layer, not the primary
confirmation mechanism.

### Proposed Behavior

Choose one or combine both:

- `:delete` asks for confirmation before removing the selected file.
- `:delete!` deletes immediately for users who explicitly want the fast path.

For CLI deletion, consider:

```sh
zake delete "Note Title"
zake delete "Note Title" --yes
```

Without `--yes`, the CLI can prompt interactively when stdin is a terminal.

### Implementation Notes

- Add a confirmation mode in the TUI, for example
  `Mode::ConfirmDelete { note_path }`.
- Show the note title and path in the status area.
- Accept `y` / `Y` to confirm and `n`, `N`, or `Esc` to cancel.
- Keep the existing `note::delete_note` function as the final filesystem action.

### Success Criteria

- A stray `:delete` cannot silently remove a note.
- Power users still have a deliberate immediate path.
- Confirmation copy includes enough context to avoid deleting the wrong note.

## 3. Graph Health Commands

### Why

Zake already indexes titles, tags, types, outgoing links, backlinks, and broken
links. Turning that index into explicit health commands would make Zake useful
as a notebook maintenance tool.

This fits the project especially well because it surfaces structure without
owning the content.

### Proposed Commands

```sh
zake links "Note Title"
zake backlinks "Note Title"
zake broken
zake orphans
```

Potential TUI commands:

```text
:links
:backlinks
:broken
:orphans
```

### Command Details

`zake links <note>`:

- Lists metadata links and inline links for one note.
- Indicates whether each target resolves to an existing note.
- Marks external links separately.

`zake backlinks <note>`:

- Lists notes that link to the selected or requested note.
- Shows whether the backlink came from frontmatter metadata, body links, or both
  if that distinction is later tracked.

`zake broken`:

- Lists notes with unresolved internal links.
- Includes the missing target text.
- Returns a non-zero exit code when broken links exist, making it useful in CI.

`zake orphans`:

- Lists notes with no outgoing internal links and no backlinks.
- Optionally ignores specific types such as `daily`, `archive`, or `reference`
  through config later.

### Implementation Notes

- Most of the required data already exists in `NoteIndex`.
- `orphans` may need helper methods for internal outgoing link counts.
- Consider relative paths in output to make command results easier to scan.

### Success Criteria

- Users can quickly see where the notebook graph is weak.
- Broken links can be checked from scripts or Git hooks.
- The TUI can expose graph health without becoming a graph visualizer.

## 4. Rename Link Repair

### Why

Renaming a note updates the note title and file path, but links from other notes
may still point to the old title. For notebooks that rely on wiki links or
frontmatter links, this makes rename operations feel incomplete.

Zake can provide a careful repair tool while still leaving Markdown as the source
of truth.

### Proposed Behavior

```sh
zake rename "Old Title" "New Title" --update-links
```

TUI option:

```text
:rename New Title --update-links
```

When enabled, Zake should update references from the old title to the new title:

- Frontmatter `links` entries.
- Wiki links like `[[Old Title]]`.
- Wiki links with labels like `[[Old Title|label]]`, preserving the label.

Markdown links should be handled more conservatively because their targets may be
paths, URLs, anchors, or arbitrary text.

### Implementation Notes

- Start with exact title matches only.
- Use structured frontmatter parsing for metadata links.
- For body replacements, avoid broad global string replacement. Target wiki-link
  syntax specifically.
- Consider a dry-run mode before applying edits:

```sh
zake rename "Old Title" "New Title" --update-links --dry-run
```

### Success Criteria

- Renaming a note does not leave obvious broken internal links.
- Link repair is opt-in and predictable.
- Labeled wiki links preserve their visible labels.

## 5. Better Command Parsing

### Why

The TUI command parser currently splits on whitespace. This keeps the first
implementation simple, but it prevents natural inputs like multi-word tags and
links.

This becomes especially noticeable in commands such as:

```text
:tag "machine learning" rust
:link "Project Notes" "Reading List"
```

### Proposed Behavior

- Support quoted arguments in TUI commands.
- Support escaped quotes inside quoted strings.
- Preserve current simple commands exactly as they work today.
- Return clear parse errors for unterminated quotes.

### Implementation Notes

- Add a small shell-like argument parser instead of using
  `input.split_whitespace()`.
- Keep the parser local and well-tested.
- Consider whether CLI and TUI command parsing should share helpers where useful.

### Success Criteria

- Users can enter multi-word tags, types, folders, and links.
- Existing commands remain compatible.
- Error messages explain malformed command input clearly.

## 6. Notebook Ritual Commands

### Why

Many note systems become valuable through repeatable capture rituals: daily
notes, weekly reviews, meeting notes, project logs. Zake can support these
without imposing a full workflow.

The right version of this feature creates predictable Markdown files and then
hands control back to the user's editor.

### Proposed Commands

```sh
zake today
zake week
```

Potential future commands:

```sh
zake daily 2026-06-05
zake weekly 2026-W23
```

### Proposed Behavior

`zake today`:

- Creates or opens today's note.
- Uses a stable path such as `daily/YYYY-MM-DD.md`.
- Sets type to `daily`.
- Optionally opens the note in `$EDITOR`.

`zake week`:

- Creates or opens the current weekly note.
- Uses a stable path such as `weekly/YYYY-Www.md`.
- Sets type to `weekly`.

### Configuration Ideas

Later, `.zake/config.toml` could support:

```toml
[templates]
daily = ".zake/templates/daily.md"
weekly = ".zake/templates/weekly.md"

[rituals]
daily_dir = "daily"
weekly_dir = "weekly"
open_after_create = true
```

### Implementation Notes

- Start without templates: create simple Markdown stubs with frontmatter.
- Add templates after the basic commands are stable.
- Avoid making daily or weekly notes special in the core data model; they can
  just be ordinary notes with predictable type and path.

### Success Criteria

- Users can start a daily or weekly note with one command.
- Generated files remain ordinary Markdown.
- The feature encourages capture without locking users into a workflow.

## Suggested Order

1. Interactive fuzzy picker.
2. Safer destructive actions.
3. Graph health commands.
4. Better command parsing.
5. Rename link repair.
6. Notebook ritual commands.

The first three features provide the strongest immediate value while staying
closest to Zake's existing architecture.

