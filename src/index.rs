use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ignore::WalkBuilder;

use crate::note::{Note, parse_note};
use crate::notebook::Notebook;

#[derive(Debug, Clone, Default)]
pub struct NoteIndex {
    pub notes: Vec<Note>,
    pub by_title: BTreeMap<String, usize>,
    pub by_tag: BTreeMap<String, Vec<usize>>,
    pub by_type: BTreeMap<String, Vec<usize>>,
    pub backlinks: BTreeMap<String, Vec<usize>>,
    pub broken_links: BTreeMap<PathBuf, Vec<String>>,
    pub parse_errors: Vec<(PathBuf, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub index: usize,
    pub score: i64,
}

impl NoteIndex {
    pub fn build(notebook: &Notebook) -> Self {
        let mut index = Self::default();
        let notes_root = notebook.notes_root();

        for entry in WalkBuilder::new(&notes_root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !is_markdown(path) || path.starts_with(notebook.root.join(".zake")) {
                continue;
            }
            match parse_note(path) {
                Ok(note) => {
                    let note_idx = index.notes.len();
                    index
                        .by_title
                        .insert(note.meta.title.to_lowercase(), note_idx);
                    for tag in &note.meta.tags {
                        index
                            .by_tag
                            .entry(tag.to_lowercase())
                            .or_default()
                            .push(note_idx);
                    }
                    index
                        .by_type
                        .entry(note.meta.kind.to_lowercase())
                        .or_default()
                        .push(note_idx);
                    index.notes.push(note);
                }
                Err(err) => index
                    .parse_errors
                    .push((path.to_path_buf(), err.to_string())),
            }
        }

        index.rebuild_links();
        index
    }

    pub fn fuzzy_notes(&self, query: &str) -> Vec<Match> {
        let matcher = SkimMatcherV2::default();
        let mut matches: Vec<_> = self
            .notes
            .iter()
            .enumerate()
            .filter_map(|(index, note)| {
                let haystack = format!("{} {}", note.meta.title, note.path.display());
                matcher
                    .fuzzy_match(&haystack, query)
                    .map(|score| Match { index, score })
            })
            .collect();
        matches.sort_by(|a, b| b.score.cmp(&a.score));
        matches
    }

    pub fn notes_for_tag(&self, tag: &str) -> Vec<&Note> {
        self.by_tag
            .get(&tag.to_lowercase())
            .into_iter()
            .flatten()
            .filter_map(|idx| self.notes.get(*idx))
            .collect()
    }

    pub fn notes_for_type(&self, kind: &str) -> Vec<&Note> {
        self.by_type
            .get(&kind.to_lowercase())
            .into_iter()
            .flatten()
            .filter_map(|idx| self.notes.get(*idx))
            .collect()
    }

    fn rebuild_links(&mut self) {
        let titles: BTreeSet<_> = self.by_title.keys().cloned().collect();

        for (idx, note) in self.notes.iter().enumerate() {
            let mut missing = Vec::new();
            for link in note.meta.links.iter().chain(note.inline_links.iter()) {
                if is_external(link) || titles.contains(&link.to_lowercase()) {
                    self.backlinks
                        .entry(link.to_lowercase())
                        .or_default()
                        .push(idx);
                } else {
                    missing.push(link.clone());
                }
            }
            if !missing.is_empty() {
                self.broken_links.insert(note.path.clone(), missing);
            }
        }
    }
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext, "md" | "markdown"))
}

fn is_external(link: &str) -> bool {
    link.starts_with("http://") || link.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::note::create_note;
    use crate::notebook::Notebook;

    use super::*;

    #[test]
    fn indexes_tags_types_and_broken_links() {
        let dir = tempdir().unwrap();
        let notebook = Notebook::init(dir.path()).unwrap();
        let note = create_note(&notebook, "Alpha").unwrap();
        fs::write(
            &note.path,
            r#"---
title: Alpha
type: idea
tags:
- rust
links:
- Missing
created_at: 2026-01-01T00:00:00Z
updated_at: 2026-01-01T00:00:00Z
---

See [[Also Missing]].
"#,
        )
        .unwrap();

        let index = NoteIndex::build(&notebook);
        assert_eq!(index.notes_for_tag("rust").len(), 1);
        assert_eq!(index.notes_for_type("idea").len(), 1);
        assert_eq!(index.broken_links[&note.path].len(), 2);
    }
}
