use std::env;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, anyhow};
use chrono::Local;

use crate::git::{self, GitStatus};
use crate::index::{LinkSource, NoteIndex};
use crate::note::{self, Note};
use crate::notebook::Notebook;
use crate::search::{self, SearchHit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Notes,
    Metadata,
    Git,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Columns,
    Workbench,
    GitWide,
    Stack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Command(String),
    Find {
        query: String,
        previous_selection: usize,
        selected_match: usize,
    },
    ConfirmDelete {
        note: Note,
    },
}

#[derive(Debug)]
pub struct AppState {
    pub notebook: Notebook,
    pub index: NoteIndex,
    pub git: GitStatus,
    pub history: Vec<String>,
    pub search_hits: Vec<SearchHit>,
    pub selected_note: usize,
    pub selected_git: usize,
    pub selected_search: usize,
    pub focus: FocusPane,
    pub layout: LayoutMode,
    pub zoomed: Option<FocusPane>,
    pub mode: Mode,
    pub message: String,
    pub should_quit: bool,
}

impl AppState {
    pub fn load(notebook: Notebook) -> Self {
        let index = NoteIndex::build(&notebook);
        let git = git::status(&notebook.root).unwrap_or_default();
        let history = git::history(&notebook.root, 5).unwrap_or_default();
        Self {
            notebook,
            index,
            git,
            history,
            search_hits: Vec::new(),
            selected_note: 0,
            selected_git: 0,
            selected_search: 0,
            focus: FocusPane::Notes,
            layout: LayoutMode::Workbench,
            zoomed: None,
            mode: Mode::Normal,
            message: "Enter edit | Tab pane | w layout | m zoom | ? help".to_string(),
            should_quit: false,
        }
    }

    pub fn selected_note(&self) -> Option<&Note> {
        self.index.notes.get(self.selected_note)
    }

    pub fn refresh(&mut self) {
        self.index = NoteIndex::build(&self.notebook);
        self.git = git::status(&self.notebook.root).unwrap_or_default();
        self.history = git::history(&self.notebook.root, 5).unwrap_or_default();
        self.selected_note = self
            .selected_note
            .min(self.index.notes.len().saturating_sub(1));
        self.selected_git = self
            .selected_git
            .min(self.git.files.len().saturating_sub(1));
        self.selected_search = self
            .selected_search
            .min(self.search_hits.len().saturating_sub(1));
    }

    pub fn move_selection(&mut self, delta: isize) {
        match self.focus {
            FocusPane::Notes | FocusPane::Metadata => {
                self.selected_note = move_index(self.selected_note, self.index.notes.len(), delta);
            }
            FocusPane::Git => {
                self.selected_git = move_index(self.selected_git, self.git.files.len(), delta);
            }
            FocusPane::Search => {
                let len = if self.search_hits.is_empty() {
                    self.history.len()
                } else {
                    self.search_hits.len()
                };
                self.selected_search = move_index(self.selected_search, len, delta);
            }
        }
    }

    pub fn next_focus(&mut self) {
        self.focus = match self.focus {
            FocusPane::Notes => FocusPane::Metadata,
            FocusPane::Metadata => FocusPane::Git,
            FocusPane::Git => FocusPane::Search,
            FocusPane::Search => FocusPane::Notes,
        };
    }

    pub fn previous_focus(&mut self) {
        self.focus = match self.focus {
            FocusPane::Notes => FocusPane::Search,
            FocusPane::Metadata => FocusPane::Notes,
            FocusPane::Git => FocusPane::Metadata,
            FocusPane::Search => FocusPane::Git,
        };
    }

    pub fn cycle_layout(&mut self) {
        self.layout = match self.layout {
            LayoutMode::Columns => LayoutMode::Workbench,
            LayoutMode::Workbench => LayoutMode::GitWide,
            LayoutMode::GitWide => LayoutMode::Stack,
            LayoutMode::Stack => LayoutMode::Columns,
        };
        self.zoomed = None;
        self.message = format!("layout: {}", self.layout.label());
    }

    pub fn toggle_zoom(&mut self) {
        self.zoomed = match self.zoomed {
            Some(pane) if pane == self.focus => None,
            _ => Some(self.focus),
        };
        self.message = if self.zoomed.is_some() {
            format!("zoomed {}", self.focus.label())
        } else {
            "zoom off".to_string()
        };
    }

    pub fn edit_selected(&mut self) -> Result<()> {
        let selected = self
            .selected_note()
            .ok_or_else(|| anyhow!("no note selected"))?
            .clone();
        open_external_editor(&selected.path).context("open external editor")?;
        self.message = "returned from editor".to_string();
        self.refresh();
        Ok(())
    }

    pub fn open_selected_search_hit(&mut self) -> Result<()> {
        let hit = self
            .search_hits
            .get(self.selected_search)
            .ok_or_else(|| anyhow!("no search hit selected"))?
            .clone();
        self.select_note_by_path(&hit.path)
            .ok_or_else(|| anyhow!("search hit is not an indexed note"))?;
        self.edit_selected()
    }

    pub fn toggle_selected_git(&mut self) -> Result<()> {
        let file = self
            .git
            .files
            .get(self.selected_git)
            .ok_or_else(|| anyhow!("no Git file selected"))?;
        if file.staged.is_some() && file.unstaged.is_none() && !file.is_untracked() {
            git::unstage(&self.notebook.root, &file.path)?;
            self.message = format!("unstaged {}", file.path.display());
        } else {
            git::stage(&self.notebook.root, &file.path)?;
            self.message = format!("staged {}", file.path.display());
        }
        self.refresh();
        Ok(())
    }

    pub fn snapshot_now(&mut self) -> Result<()> {
        let message = format!("Snapshot {}", Local::now().format("%Y-%m-%d %H:%M"));
        git::snapshot(&self.notebook.root, &message)?;
        self.message = format!("snapshotted: {message}");
        self.refresh();
        Ok(())
    }

    pub fn command_for_selected_link(&self) -> String {
        let mut command = String::from("link ");
        if let Some(note) = self.selected_note() {
            command.push('"');
            command.push_str(&note.meta.title.replace('"', "\\\""));
            command.push('"');
            command.push(' ');
        }
        command
    }

    pub fn select_note_by_path(&mut self, path: &PathBuf) -> Option<()> {
        let absolute = if path.is_absolute() {
            path.clone()
        } else {
            self.notebook.root.join(path)
        };
        let idx = self
            .index
            .notes
            .iter()
            .position(|note| note.path == *path || note.path == absolute)?;
        self.selected_note = idx;
        self.focus = FocusPane::Notes;
        Some(())
    }

    pub fn run_command(&mut self, input: &str) -> Result<()> {
        let mut args = parse_command_args(input)?;
        if args.is_empty() {
            return Ok(());
        }
        let command = args.remove(0);

        match command.as_str() {
            "new" => {
                let title = args.join(" ");
                if title.is_empty() {
                    return Err(anyhow!("usage: new <title>"));
                }
                let note = note::create_note(&self.notebook, &title)?;
                self.message = format!("created {}", note.path.display());
                self.refresh();
            }
            "rename" => {
                let update_links = remove_flag(&mut args, "--update-links");
                let dry_run = remove_flag(&mut args, "--dry-run");
                let title = args.join(" ");
                if title.is_empty() {
                    return Err(anyhow!("usage: rename <title>"));
                }
                let selected = self
                    .selected_note()
                    .ok_or_else(|| anyhow!("no note selected"))?
                    .clone();
                if update_links {
                    let repair = note::rename_note_with_link_repair(
                        &selected,
                        &title,
                        &self.notebook,
                        dry_run,
                    )?;
                    if dry_run {
                        self.message = format!(
                            "would rename to {} and update {} files",
                            repair.renamed_path.display(),
                            repair.updated_link_files.len()
                        );
                    } else {
                        self.message = format!(
                            "renamed to {} and updated {} files",
                            repair.renamed_path.display(),
                            repair.updated_link_files.len()
                        );
                        self.refresh();
                    }
                } else {
                    let path = note::rename_note(&selected, &title, &self.notebook)?;
                    self.message = format!("renamed to {}", path.display());
                    self.refresh();
                }
            }
            "move" => {
                let folder = PathBuf::from(args.join(" "));
                if folder.as_os_str().is_empty() {
                    return Err(anyhow!("usage: move <folder>"));
                }
                let selected = self
                    .selected_note()
                    .ok_or_else(|| anyhow!("no note selected"))?
                    .clone();
                let path = note::move_note(&selected, &self.notebook, &folder)?;
                self.message = format!("moved to {}", path.display());
                self.refresh();
            }
            "delete!" => {
                let selected = self
                    .selected_note()
                    .ok_or_else(|| anyhow!("no note selected"))?
                    .clone();
                note::delete_note(&selected)?;
                self.message = format!("deleted {}", selected.path.display());
                self.refresh();
            }
            "delete" => {
                let selected = self
                    .selected_note()
                    .ok_or_else(|| anyhow!("no note selected"))?
                    .clone();
                self.mode = Mode::ConfirmDelete { note: selected };
                self.message = "confirm delete with y, cancel with n or Esc".to_string();
            }
            "tag" => {
                let tags = args;
                let selected = self
                    .selected_note()
                    .ok_or_else(|| anyhow!("no note selected"))?
                    .clone();
                note::update_metadata(&selected.path, |meta| meta.tags = tags)?;
                self.message = "updated tags".to_string();
                self.refresh();
            }
            "type" => {
                let kind = args.join(" ");
                if kind.is_empty() {
                    return Err(anyhow!("usage: type <kind>"));
                }
                let selected = self
                    .selected_note()
                    .ok_or_else(|| anyhow!("no note selected"))?
                    .clone();
                note::update_metadata(&selected.path, |meta| meta.kind = kind)?;
                self.message = "updated type".to_string();
                self.refresh();
            }
            "link" => {
                let links = args;
                let selected = self
                    .selected_note()
                    .ok_or_else(|| anyhow!("no note selected"))?
                    .clone();
                note::update_metadata(&selected.path, |meta| meta.links = links)?;
                self.message = "updated links".to_string();
                self.refresh();
            }
            "open" => {
                self.edit_selected()?;
            }
            "search" => {
                let query = args.join(" ");
                self.search_hits = search::ripgrep(&self.notebook.root, &query)?;
                self.selected_search = 0;
                self.focus = FocusPane::Search;
                self.message = format!("{} search hits", self.search_hits.len());
            }
            "stage" => {
                if let Some(file) = self.git.files.get(self.selected_git) {
                    git::stage(&self.notebook.root, &file.path)?;
                    self.message = format!("staged {}", file.path.display());
                    self.refresh();
                }
            }
            "unstage" => {
                if let Some(file) = self.git.files.get(self.selected_git) {
                    git::unstage(&self.notebook.root, &file.path)?;
                    self.message = format!("unstaged {}", file.path.display());
                    self.refresh();
                }
            }
            "stage-all" => {
                git::stage_all(&self.notebook.root)?;
                self.message = "staged all changes".to_string();
                self.refresh();
            }
            "commit" => {
                let message = args.join(" ");
                if message.is_empty() {
                    return Err(anyhow!("usage: commit <message>"));
                }
                git::commit(&self.notebook.root, &message)?;
                self.message = "committed changes".to_string();
                self.refresh();
            }
            "status" => {
                self.refresh();
                self.focus = FocusPane::Git;
                self.message = format!("{} changes", self.git.files.len());
            }
            "history" => {
                let limit = args
                    .first()
                    .map(|arg| arg.parse::<usize>())
                    .transpose()
                    .context("parse history limit")?
                    .unwrap_or(10);
                self.search_hits.clear();
                self.history = git::history(&self.notebook.root, limit)?;
                if self.history.is_empty() {
                    self.history.push("no history".to_string());
                }
                self.selected_search = 0;
                self.focus = FocusPane::Search;
                self.message = format!("{} history entries", self.history.len());
            }
            "diff" => {
                let target = if args.is_empty() {
                    self.selected_note().map(|note| note.path.clone())
                } else {
                    Some(PathBuf::from(args.join(" ")))
                };
                let diff = git::diff(&self.notebook.root, target.as_deref())?;
                self.search_hits.clear();
                self.history = if diff.is_empty() {
                    vec!["no diff".to_string()]
                } else {
                    diff.lines().map(str::to_string).collect()
                };
                self.selected_search = 0;
                self.focus = FocusPane::Search;
                self.message = "diff loaded".to_string();
            }
            "snapshot" => {
                let message = args.join(" ");
                if message.is_empty() {
                    return Err(anyhow!("usage: snapshot <message>"));
                }
                git::snapshot(&self.notebook.root, &message)?;
                self.message = "snapshotted notebook".to_string();
                self.refresh();
            }
            "links" => {
                let selected = self
                    .selected_note()
                    .ok_or_else(|| anyhow!("no note selected"))?
                    .clone();
                self.show_links(&selected);
            }
            "backlinks" => {
                let selected = self
                    .selected_note()
                    .ok_or_else(|| anyhow!("no note selected"))?
                    .clone();
                self.show_backlinks(&selected);
            }
            "broken" => self.show_broken(),
            "orphans" => self.show_orphans(),
            "refresh" => {
                self.refresh();
                self.message = "refreshed".to_string();
            }
            "help" | "?" => {
                self.message = "Commands: new, rename, move, delete, delete!, tag, type, link, open, search, links, backlinks, broken, orphans, status, stage, unstage, stage-all, commit, history, diff, snapshot, refresh".to_string();
            }
            "quit" | "q" => self.should_quit = true,
            other => return Err(anyhow!("unknown command: {other}")),
        }

        Ok(())
    }

    pub fn find_matches(&self, query: &str) -> Vec<usize> {
        if query.is_empty() {
            return (0..self.index.notes.len()).collect();
        }
        self.index
            .fuzzy_notes(query)
            .into_iter()
            .map(|matched| matched.index)
            .collect()
    }

    pub fn show_links(&mut self, note: &Note) {
        let statuses = self.index.link_statuses(note);
        self.search_hits.clear();
        self.history = statuses
            .iter()
            .map(|status| {
                let source = match status.source {
                    LinkSource::Metadata => "meta",
                    LinkSource::Inline => "inline",
                };
                let state = if status.external {
                    "external"
                } else if status.resolved {
                    "ok"
                } else {
                    "broken"
                };
                format!("{source}\t{state}\t{}", status.target)
            })
            .collect();
        if self.history.is_empty() {
            self.history.push("no links".to_string());
        }
        self.selected_search = 0;
        self.focus = FocusPane::Search;
        self.message = format!("{} links", statuses.len());
    }

    pub fn show_backlinks(&mut self, note: &Note) {
        let backlinks = self.index.backlinks_for(&note.meta.title);
        self.search_hits.clear();
        self.history = backlinks
            .iter()
            .map(|backlink| format!("{}\t{}", backlink.meta.title, backlink.path.display()))
            .collect();
        if self.history.is_empty() {
            self.history.push("no backlinks".to_string());
        }
        self.selected_search = 0;
        self.focus = FocusPane::Search;
        self.message = format!("{} backlinks", backlinks.len());
    }

    pub fn show_broken(&mut self) {
        let entries = self.index.broken_entries();
        self.search_hits.clear();
        self.history = entries
            .iter()
            .map(|(note, target)| format!("{}\tmissing {}", note.meta.title, target))
            .collect();
        if self.history.is_empty() {
            self.history.push("no broken links".to_string());
        }
        self.selected_search = 0;
        self.focus = FocusPane::Search;
        self.message = format!("{} broken links", entries.len());
    }

    pub fn show_orphans(&mut self) {
        let orphans = self.index.orphan_notes();
        self.search_hits.clear();
        self.history = orphans
            .iter()
            .map(|note| format!("{}\t{}", note.meta.title, note.path.display()))
            .collect();
        if self.history.is_empty() {
            self.history.push("no orphans".to_string());
        }
        self.selected_search = 0;
        self.focus = FocusPane::Search;
        self.message = format!("{} orphans", orphans.len());
    }
}

impl FocusPane {
    pub fn label(self) -> &'static str {
        match self {
            FocusPane::Notes => "notes",
            FocusPane::Metadata => "metadata",
            FocusPane::Git => "git",
            FocusPane::Search => "search",
        }
    }
}

impl LayoutMode {
    pub fn label(self) -> &'static str {
        match self {
            LayoutMode::Columns => "columns",
            LayoutMode::Workbench => "workbench",
            LayoutMode::GitWide => "git-wide",
            LayoutMode::Stack => "stack",
        }
    }
}

fn move_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    current.saturating_add_signed(delta).min(len - 1)
}

fn open_external_editor(path: &PathBuf) -> Result<()> {
    let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = Command::new(editor)
        .arg(path)
        .status()
        .with_context(|| format!("open {}", path.display()))?;
    if !status.success() {
        return Err(anyhow!("editor exited unsuccessfully"));
    }
    Ok(())
}

fn remove_flag(args: &mut Vec<String>, flag: &str) -> bool {
    let found = args.iter().any(|arg| arg == flag);
    args.retain(|arg| arg != flag);
    found
}

pub fn parse_command_args(input: &str) -> Result<Vec<String>> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' => in_quotes = !in_quotes,
            '\\' if in_quotes => match chars.next() {
                Some('"') => current.push('"'),
                Some('\\') => current.push('\\'),
                Some(other) => {
                    current.push('\\');
                    current.push(other);
                }
                None => current.push('\\'),
            },
            ch if ch.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            ch => current.push(ch),
        }
    }

    if in_quotes {
        return Err(anyhow!("unterminated quoted argument"));
    }
    if !current.is_empty() {
        args.push(current);
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::parse_command_args;

    #[test]
    fn parses_quoted_command_arguments() {
        assert_eq!(
            parse_command_args(r#"tag "machine learning" rust"#).unwrap(),
            vec!["tag", "machine learning", "rust"]
        );
        assert_eq!(
            parse_command_args(r#"link "Project \"Alpha\"""#).unwrap(),
            vec!["link", r#"Project "Alpha""#]
        );
        assert!(parse_command_args(r#"tag "oops"#).is_err());
    }
}
