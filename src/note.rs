use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::notebook::Notebook;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoteMeta {
    pub title: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub links: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NoteMeta {
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            title: title.into(),
            kind: "note".to_string(),
            tags: Vec::new(),
            links: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub path: PathBuf,
    pub meta: NoteMeta,
    pub inline_links: Vec<String>,
}

pub fn slugify(title: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;

    for ch in title.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            out.push(ch);
            pending_dash = false;
        } else if ch.is_whitespace() || matches!(ch, '-' | '_' | '.') {
            pending_dash = true;
        }
    }

    if out.is_empty() {
        "untitled".to_string()
    } else {
        out
    }
}

pub fn create_note(notebook: &Notebook, title: &str) -> Result<Note> {
    create_note_with_metadata(notebook, title, None, Vec::new(), Vec::new())
}

pub fn create_note_with_metadata(
    notebook: &Notebook,
    title: &str,
    kind: Option<String>,
    tags: Vec<String>,
    links: Vec<String>,
) -> Result<Note> {
    let slug = slugify(title);
    let mut path = notebook.notes_root().join(format!("{slug}.md"));
    let mut suffix = 2;
    while path.exists() {
        path = notebook.notes_root().join(format!("{slug}-{suffix}.md"));
        suffix += 1;
    }

    let mut meta = NoteMeta::new(title);
    if let Some(kind) = kind {
        meta.kind = kind;
    }
    meta.tags = tags;
    meta.links = links;
    write_new_note(&path, &meta)?;
    Ok(Note {
        path,
        meta,
        inline_links: Vec::new(),
    })
}

pub fn create_or_open_note_at(
    notebook: &Notebook,
    relative_path: &Path,
    title: &str,
    kind: &str,
) -> Result<Note> {
    let path = notebook.notes_root().join(relative_path);
    if path.exists() {
        return parse_note(path);
    }

    let mut meta = NoteMeta::new(title);
    meta.kind = kind.to_string();
    write_new_note(&path, &meta)?;
    parse_note(path)
}

pub fn rename_note(note: &Note, new_title: &str, notebook: &Notebook) -> Result<PathBuf> {
    let new_slug = slugify(new_title);
    let new_path = notebook.notes_root().join(format!("{new_slug}.md"));
    if new_path != note.path && new_path.exists() {
        bail!("target note already exists: {}", new_path.display());
    }

    let mut meta = note.meta.clone();
    meta.title = new_title.to_string();
    meta.updated_at = Utc::now();
    update_frontmatter(&note.path, &meta)?;
    if new_path != note.path {
        fs::rename(&note.path, &new_path)
            .with_context(|| format!("rename {} to {}", note.path.display(), new_path.display()))?;
    }
    Ok(new_path)
}

pub fn rename_note_with_link_repair(
    note: &Note,
    new_title: &str,
    notebook: &Notebook,
    dry_run: bool,
) -> Result<RenameRepair> {
    let old_title = note.meta.title.clone();
    let new_slug = slugify(new_title);
    let new_path = notebook.notes_root().join(format!("{new_slug}.md"));
    if new_path != note.path && new_path.exists() {
        bail!("target note already exists: {}", new_path.display());
    }

    let mut changed_paths = repair_links(notebook, &old_title, new_title, dry_run)?;
    if !dry_run {
        let renamed_path = rename_note(note, new_title, notebook)?;
        for path in &mut changed_paths {
            if path == &note.path {
                *path = renamed_path.clone();
            }
        }
        Ok(RenameRepair {
            renamed_path,
            updated_link_files: changed_paths,
        })
    } else {
        Ok(RenameRepair {
            renamed_path: new_path,
            updated_link_files: changed_paths,
        })
    }
}

pub fn move_note(note: &Note, notebook: &Notebook, folder: &Path) -> Result<PathBuf> {
    let target_dir = notebook.notes_root().join(folder);
    fs::create_dir_all(&target_dir).with_context(|| format!("create {}", target_dir.display()))?;
    let file_name = note
        .path
        .file_name()
        .ok_or_else(|| anyhow!("note path has no file name"))?;
    let target = target_dir.join(file_name);
    if target.exists() {
        bail!("target note already exists: {}", target.display());
    }
    fs::rename(&note.path, &target)
        .with_context(|| format!("move {} to {}", note.path.display(), target.display()))?;
    Ok(target)
}

pub fn delete_note(note: &Note) -> Result<()> {
    fs::remove_file(&note.path).with_context(|| format!("delete {}", note.path.display()))
}

pub fn update_metadata(path: &Path, edit: impl FnOnce(&mut NoteMeta)) -> Result<()> {
    let (mut meta, _) = read_note_parts(path)?;
    edit(&mut meta);
    meta.updated_at = Utc::now();
    update_frontmatter(path, &meta)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameRepair {
    pub renamed_path: PathBuf,
    pub updated_link_files: Vec<PathBuf>,
}

pub fn parse_note(path: impl AsRef<Path>) -> Result<Note> {
    let path = path.as_ref();
    let (meta, body) = read_note_parts(path)?;
    let inline_links = extract_inline_links(&body);
    Ok(Note {
        path: path.to_path_buf(),
        meta,
        inline_links,
    })
}

fn write_new_note(path: &Path, meta: &NoteMeta) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let frontmatter = serde_yaml::to_string(meta)?;
    let contents = format!("---\n{frontmatter}---\n\n# {}\n", meta.title);
    fs::write(path, contents).with_context(|| format!("write {}", path.display()))
}

fn update_frontmatter(path: &Path, meta: &NoteMeta) -> Result<()> {
    let (_, body) = read_note_parts(path)?;
    write_note_parts(path, meta, &body)
}

fn write_note_parts(path: &Path, meta: &NoteMeta, body: &str) -> Result<()> {
    let frontmatter = serde_yaml::to_string(meta)?;
    fs::write(path, format!("---\n{frontmatter}---\n{body}"))
        .with_context(|| format!("write {}", path.display()))
}

fn read_note_parts(path: &Path) -> Result<(NoteMeta, String)> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let body_start = raw
        .strip_prefix("---\n")
        .ok_or_else(|| anyhow!("{} is missing YAML frontmatter", path.display()))?;
    let end = body_start
        .find("\n---")
        .ok_or_else(|| anyhow!("{} has unterminated YAML frontmatter", path.display()))?;
    let yaml = &body_start[..end];
    let body = body_start[end + "\n---".len()..].to_string();
    let meta = serde_yaml::from_str(yaml)
        .with_context(|| format!("parse YAML frontmatter in {}", path.display()))?;
    Ok((meta, body))
}

pub fn extract_inline_links(body: &str) -> Vec<String> {
    let mut links = BTreeSet::new();
    for line in body.lines() {
        extract_wiki_links(line, &mut links);
        extract_markdown_links(line, &mut links);
    }
    links.into_iter().collect()
}

fn extract_wiki_links(line: &str, links: &mut BTreeSet<String>) {
    let mut rest = line;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            break;
        };
        let target = rest[..end].split('|').next().unwrap_or("").trim();
        if !target.is_empty() {
            links.insert(target.to_string());
        }
        rest = &rest[end + 2..];
    }
}

fn extract_markdown_links(line: &str, links: &mut BTreeSet<String>) {
    let mut rest = line;
    while let Some(label_end) = rest.find("](") {
        rest = &rest[label_end + 2..];
        let Some(end) = rest.find(')') else {
            break;
        };
        let target = rest[..end].trim();
        if !target.is_empty() {
            links.insert(target.to_string());
        }
        rest = &rest[end + 1..];
    }
}

fn repair_links(
    notebook: &Notebook,
    old_title: &str,
    new_title: &str,
    dry_run: bool,
) -> Result<Vec<PathBuf>> {
    let mut changed_paths = Vec::new();
    let index = crate::index::NoteIndex::build(notebook);

    for note in index.notes {
        let (mut meta, body) = read_note_parts(&note.path)?;
        let mut changed = false;

        for link in &mut meta.links {
            if link == old_title {
                *link = new_title.to_string();
                changed = true;
            }
        }

        let repaired_body = replace_wiki_links(&body, old_title, new_title);
        if repaired_body != body {
            changed = true;
        }

        if changed {
            changed_paths.push(note.path.clone());
            if !dry_run {
                meta.updated_at = Utc::now();
                write_note_parts(&note.path, &meta, &repaired_body)?;
            }
        }
    }

    Ok(changed_paths)
}

fn replace_wiki_links(body: &str, old_title: &str, new_title: &str) -> String {
    let mut output = String::with_capacity(body.len());
    let mut rest = body;

    while let Some(start) = rest.find("[[") {
        output.push_str(&rest[..start]);
        output.push_str("[[");
        rest = &rest[start + 2..];

        let Some(end) = rest.find("]]") else {
            output.push_str(rest);
            return output;
        };

        let inner = &rest[..end];
        let (target, label) = inner
            .split_once('|')
            .map_or((inner, None), |(target, label)| (target, Some(label)));
        if target.trim() == old_title {
            output.push_str(new_title);
            if let Some(label) = label {
                output.push('|');
                output.push_str(label);
            }
        } else {
            output.push_str(inner);
        }
        output.push_str("]]");
        rest = &rest[end + 2..];
    }

    output.push_str(rest);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    use crate::index::NoteIndex;

    #[test]
    fn slugify_title() {
        assert_eq!(slugify("Hello, Zake Notes!"), "hello-zake-notes");
        assert_eq!(slugify("..."), "untitled");
    }

    #[test]
    fn extracts_markdown_and_wiki_links() {
        let links =
            extract_inline_links("See [[Daily Note|today]] and [Rust](https://rust-lang.org).");
        assert_eq!(links, vec!["Daily Note", "https://rust-lang.org"]);
    }

    #[test]
    fn repairs_exact_wiki_links_and_metadata_links() {
        let dir = tempdir().unwrap();
        let notebook = Notebook::init(dir.path()).unwrap();
        let old = create_note(&notebook, "Old Title").unwrap();
        let referer = create_note_with_metadata(
            &notebook,
            "Referer",
            None,
            Vec::new(),
            vec!["Old Title".to_string()],
        )
        .unwrap();
        fs::write(
            &referer.path,
            r#"---
title: Referer
type: note
tags: []
links:
- Old Title
created_at: 2026-01-01T00:00:00Z
updated_at: 2026-01-01T00:00:00Z
---

See [[Old Title]] and [[Old Title|the old one]].
"#,
        )
        .unwrap();

        rename_note_with_link_repair(&old, "New Title", &notebook, false).unwrap();
        let repaired = parse_note(&referer.path).unwrap();
        assert_eq!(repaired.meta.links, vec!["New Title"]);
        let raw = fs::read_to_string(&referer.path).unwrap();
        assert!(raw.contains("[[New Title]]"));
        assert!(raw.contains("[[New Title|the old one]]"));
    }

    #[test]
    fn create_rename_move_delete_lifecycle() {
        let dir = tempdir().unwrap();
        let notebook = Notebook::init(dir.path()).unwrap();

        let note = create_note(&notebook, "First Note").unwrap();
        assert!(note.path.exists());
        assert_eq!(parse_note(&note.path).unwrap().meta.title, "First Note");

        let renamed_path = rename_note(&note, "Renamed Note", &notebook).unwrap();
        assert!(renamed_path.ends_with("renamed-note.md"));
        let renamed = parse_note(&renamed_path).unwrap();
        assert_eq!(renamed.meta.title, "Renamed Note");

        let moved_path = move_note(&renamed, &notebook, Path::new("archive")).unwrap();
        assert!(moved_path.ends_with("archive/renamed-note.md"));

        let moved = parse_note(&moved_path).unwrap();
        let index = NoteIndex::build(&notebook);
        assert_eq!(index.notes.len(), 1);

        delete_note(&moved).unwrap();
        assert!(!moved_path.exists());
        assert!(NoteIndex::build(&notebook).notes.is_empty());
    }
}
