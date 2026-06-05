use anyhow::Result;
use chrono::{Datelike, Local};
use clap::Parser;
use std::env;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use zake::app::AppState;
use zake::cli::{Cli, Command};
use zake::git;
use zake::index::NoteIndex;
use zake::note::{self as notes, Note};
use zake::notebook::Notebook;
use zake::search;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Init { path }) => {
            let notebook = Notebook::init(path)?;
            println!("Initialized Zake notebook at {}", notebook.root.display());
        }
        Some(Command::Doctor { path }) => {
            let notebook = Notebook::discover(path)?;
            let warnings = notebook.validate()?;
            let index = NoteIndex::build(&notebook);
            println!("Notebook: {}", notebook.root.display());
            println!("Notes: {}", index.notes.len());
            println!("Parse errors: {}", index.parse_errors.len());
            println!("Broken-link files: {}", index.broken_links.len());
            if warnings.is_empty() && index.parse_errors.is_empty() {
                println!("Doctor: ok");
            } else {
                for warning in warnings {
                    println!("Warning: {warning}");
                }
                for (path, error) in index.parse_errors {
                    println!("Parse error: {}: {error}", path.display());
                }
            }
        }
        Some(Command::New {
            title,
            kind,
            tags,
            links,
            path,
        }) => {
            let notebook = Notebook::discover(path)?;
            let created = notes::create_note_with_metadata(&notebook, &title, kind, tags, links)?;
            println!("{}", created.path.display());
        }
        Some(Command::List { path, tag, kind }) => {
            let notebook = Notebook::discover(path)?;
            let index = NoteIndex::build(&notebook);
            for note in filtered_notes(&index, tag.as_deref(), kind.as_deref()) {
                println!(
                    "{}\t{}\t{}\t{}",
                    note.path.display(),
                    note.meta.title,
                    note.meta.kind,
                    note.meta.tags.join(",")
                );
            }
        }
        Some(Command::Show { note, path }) => {
            let notebook = Notebook::discover(path)?;
            let index = NoteIndex::build(&notebook);
            let selected = resolve_note(&notebook, &index, &note)?;
            print_note(&index, &selected);
        }
        Some(Command::Rename {
            note,
            title,
            update_links,
            dry_run,
            path,
        }) => {
            let notebook = Notebook::discover(path)?;
            let index = NoteIndex::build(&notebook);
            let selected = resolve_note(&notebook, &index, &note)?;
            if update_links {
                let repair =
                    notes::rename_note_with_link_repair(&selected, &title, &notebook, dry_run)?;
                if dry_run {
                    println!("Would rename to {}", repair.renamed_path.display());
                    for path in repair.updated_link_files {
                        println!("Would update {}", path.display());
                    }
                } else {
                    println!("{}", repair.renamed_path.display());
                    for path in repair.updated_link_files {
                        println!("Updated links in {}", path.display());
                    }
                }
            } else {
                if dry_run {
                    anyhow::bail!("--dry-run requires --update-links");
                }
                let path = notes::rename_note(&selected, &title, &notebook)?;
                println!("{}", path.display());
            }
        }
        Some(Command::Move { note, folder, path }) => {
            let notebook = Notebook::discover(path)?;
            let index = NoteIndex::build(&notebook);
            let selected = resolve_note(&notebook, &index, &note)?;
            let path = notes::move_note(&selected, &notebook, &folder)?;
            println!("{}", path.display());
        }
        Some(Command::Delete { note, yes, path }) => {
            let notebook = Notebook::discover(path)?;
            let index = NoteIndex::build(&notebook);
            let selected = resolve_note(&notebook, &index, &note)?;
            if !yes && !confirm_delete(&selected)? {
                anyhow::bail!("delete cancelled");
            }
            notes::delete_note(&selected)?;
            println!("Deleted {}", selected.path.display());
        }
        Some(Command::Links { note, path }) => {
            let notebook = Notebook::discover(path)?;
            let index = NoteIndex::build(&notebook);
            let selected = resolve_note(&notebook, &index, &note)?;
            for status in index.link_statuses(&selected) {
                let source = match status.source {
                    zake::index::LinkSource::Metadata => "metadata",
                    zake::index::LinkSource::Inline => "inline",
                };
                let state = if status.external {
                    "external"
                } else if status.resolved {
                    "ok"
                } else {
                    "broken"
                };
                println!("{source}\t{state}\t{}", status.target);
            }
        }
        Some(Command::Backlinks { note, path }) => {
            let notebook = Notebook::discover(path)?;
            let index = NoteIndex::build(&notebook);
            let selected = resolve_note(&notebook, &index, &note)?;
            for backlink in index.backlinks_for(&selected.meta.title) {
                println!("{}\t{}", backlink.meta.title, backlink.path.display());
            }
        }
        Some(Command::Broken { path }) => {
            let notebook = Notebook::discover(path)?;
            let index = NoteIndex::build(&notebook);
            let entries = index.broken_entries();
            for (note, target) in &entries {
                println!("{}\t{}\t{}", note.path.display(), note.meta.title, target);
            }
            if !entries.is_empty() {
                std::process::exit(1);
            }
        }
        Some(Command::Orphans { path }) => {
            let notebook = Notebook::discover(path)?;
            let index = NoteIndex::build(&notebook);
            for note in index.orphan_notes() {
                println!("{}\t{}", note.path.display(), note.meta.title);
            }
        }
        Some(Command::Set {
            note,
            kind,
            tags,
            links,
            clear_tags,
            clear_links,
            path,
        }) => {
            if kind.is_none() && tags.is_empty() && links.is_empty() && !clear_tags && !clear_links
            {
                anyhow::bail!(
                    "usage: zake set <note> [--type <type>] [--tag <tag>...] [--link <target>...] [--clear-tags] [--clear-links]"
                );
            }
            let notebook = Notebook::discover(path)?;
            let index = NoteIndex::build(&notebook);
            let selected = resolve_note(&notebook, &index, &note)?;
            notes::update_metadata(&selected.path, |meta| {
                if let Some(kind) = kind {
                    meta.kind = kind;
                }
                if clear_tags || !tags.is_empty() {
                    meta.tags = tags;
                }
                if clear_links || !links.is_empty() {
                    meta.links = links;
                }
            })?;
            println!("Updated {}", selected.path.display());
        }
        Some(Command::Open { note, path }) => {
            let notebook = Notebook::discover(path)?;
            let index = NoteIndex::build(&notebook);
            let selected = resolve_note(&notebook, &index, &note)?;
            open_external_editor(&selected.path)?;
        }
        Some(Command::Search { query, path }) => {
            let notebook = Notebook::discover(path)?;
            for hit in search::ripgrep(&notebook.root, &query)? {
                println!("{}:{}:{}", hit.path.display(), hit.line, hit.text);
            }
        }
        Some(Command::Status { path }) => {
            let notebook = Notebook::discover(path)?;
            let status = git::status(&notebook.root)?;
            print_git_status(&status);
        }
        Some(Command::Stage { target, path }) => {
            let notebook = Notebook::discover(path)?;
            git::stage(&notebook.root, &target)?;
            println!("Staged {}", target.display());
        }
        Some(Command::Unstage { target, path }) => {
            let notebook = Notebook::discover(path)?;
            git::unstage(&notebook.root, &target)?;
            println!("Unstaged {}", target.display());
        }
        Some(Command::StageAll { path }) => {
            let notebook = Notebook::discover(path)?;
            git::stage_all(&notebook.root)?;
            println!("Staged all changes");
        }
        Some(Command::Commit { message, path }) => {
            let notebook = Notebook::discover(path)?;
            git::commit(&notebook.root, &message)?;
            println!("Committed changes");
        }
        Some(Command::History { limit, path }) => {
            let notebook = Notebook::discover(path)?;
            for line in git::history(&notebook.root, limit)? {
                println!("{line}");
            }
        }
        Some(Command::Diff { target, path }) => {
            let notebook = Notebook::discover(path)?;
            print!("{}", git::diff(&notebook.root, target.as_deref())?);
        }
        Some(Command::Snapshot { message, path }) => {
            let notebook = Notebook::discover(path)?;
            git::snapshot(&notebook.root, &message)?;
            println!("Snapshotted notebook");
        }
        Some(Command::Today { path }) => {
            let notebook = Notebook::discover(path)?;
            let today = Local::now().date_naive();
            let title = today.format("%Y-%m-%d").to_string();
            let relative = PathBuf::from("daily").join(format!("{title}.md"));
            let note = notes::create_or_open_note_at(&notebook, &relative, &title, "daily")?;
            println!("{}", note.path.display());
        }
        Some(Command::Week { path }) => {
            let notebook = Notebook::discover(path)?;
            let week = Local::now().date_naive().iso_week();
            let title = format!("{}-W{:02}", week.year(), week.week());
            let relative = PathBuf::from("weekly").join(format!("{title}.md"));
            let note = notes::create_or_open_note_at(&notebook, &relative, &title, "weekly")?;
            println!("{}", note.path.display());
        }
        None => {
            let notebook = match Notebook::discover(".") {
                Ok(notebook) => notebook,
                Err(err) => prompt_init_current_dir(err.to_string())?,
            };
            zake::tui::run(AppState::load(notebook))?;
        }
    }

    Ok(())
}

fn prompt_init_current_dir(reason: String) -> Result<Notebook> {
    println!("{reason}");
    print!("Initialize a Zake notebook here? [y/N] ");
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    if answer.trim().eq_ignore_ascii_case("y") || answer.trim().eq_ignore_ascii_case("yes") {
        Notebook::init(".")
    } else {
        anyhow::bail!("no notebook selected")
    }
}

fn confirm_delete(note: &Note) -> Result<bool> {
    if !io::stdin().is_terminal() {
        anyhow::bail!("refusing to delete without confirmation; pass --yes to delete");
    }
    print!(
        "Delete \"{}\" at {}? [y/N] ",
        note.meta.title,
        note.path.display()
    );
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(answer.trim().eq_ignore_ascii_case("y") || answer.trim().eq_ignore_ascii_case("yes"))
}

fn print_git_status(status: &git::GitStatus) {
    if status.files.is_empty() {
        println!("clean");
        return;
    }

    for file in &status.files {
        let staged = file.staged.unwrap_or(' ');
        let unstaged = file.unstaged.unwrap_or(' ');
        println!("{staged}{unstaged}\t{}", file.path.display());
    }
}

fn filtered_notes<'a>(
    index: &'a NoteIndex,
    tag: Option<&str>,
    kind: Option<&str>,
) -> impl Iterator<Item = &'a Note> {
    index.notes.iter().filter(move |note| {
        tag.is_none_or(|tag| {
            note.meta
                .tags
                .iter()
                .any(|item| item.eq_ignore_ascii_case(tag))
        }) && kind.is_none_or(|kind| note.meta.kind.eq_ignore_ascii_case(kind))
    })
}

fn resolve_note(notebook: &Notebook, index: &NoteIndex, query: &str) -> Result<Note> {
    let query_path = Path::new(query);
    for candidate in path_candidates(notebook, query_path, false) {
        if candidate.exists() {
            return notes::parse_note(candidate);
        }
    }

    let query_lower = query.to_lowercase();
    let matches = index
        .notes
        .iter()
        .filter(|note| note.meta.title.to_lowercase() == query_lower)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [note] => Ok((*note).clone()),
        [] => {
            for candidate in path_candidates(notebook, query_path, true) {
                if candidate.exists() {
                    return notes::parse_note(candidate);
                }
            }
            anyhow::bail!("no note found for `{query}`")
        }
        _ => anyhow::bail!("multiple notes found with title `{query}`; use a path"),
    }
}

fn path_candidates(
    notebook: &Notebook,
    path: &Path,
    include_extension_guess: bool,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    candidates.push(path.to_path_buf());
    candidates.push(notebook.root.join(path));
    candidates.push(notebook.notes_root().join(path));

    if include_extension_guess && path.extension().is_none() {
        candidates.push(path.with_extension("md"));
        candidates.push(notebook.root.join(path).with_extension("md"));
        candidates.push(notebook.notes_root().join(path).with_extension("md"));
    }

    candidates
}

fn print_note(index: &NoteIndex, note: &Note) {
    println!("Title: {}", note.meta.title);
    println!("Type: {}", note.meta.kind);
    println!("Tags: {}", note.meta.tags.join(", "));
    println!("Links: {}", note.meta.links.join(", "));
    if !note.inline_links.is_empty() {
        println!("Inline links: {}", note.inline_links.join(", "));
    }
    let backlinks = index
        .backlinks_for(&note.meta.title)
        .into_iter()
        .map(|note| note.meta.title.as_str())
        .collect::<Vec<_>>();
    println!("Backlinks: {}", backlinks.join(", "));
    if let Some(missing) = index.broken_links.get(&note.path) {
        println!("Broken links: {}", missing.join(", "));
    }
    println!("Created: {}", note.meta.created_at);
    println!("Updated: {}", note.meta.updated_at);
    println!("Path: {}", note.path.display());
}

fn open_external_editor(path: &Path) -> Result<()> {
    let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = ProcessCommand::new(editor).arg(path).status()?;
    if !status.success() {
        anyhow::bail!("editor exited unsuccessfully");
    }
    Ok(())
}
