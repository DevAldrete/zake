use std::env;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, anyhow};

use crate::git::{self, GitStatus};
use crate::index::NoteIndex;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Command(String),
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
            mode: Mode::Normal,
            message: "Press : for commands, ? for help, q to quit".to_string(),
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
                self.selected_search =
                    move_index(self.selected_search, self.search_hits.len(), delta);
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

    pub fn run_command(&mut self, input: &str) -> Result<()> {
        let mut words = input.split_whitespace();
        let Some(command) = words.next() else {
            return Ok(());
        };

        match command {
            "new" => {
                let title = words.collect::<Vec<_>>().join(" ");
                if title.is_empty() {
                    return Err(anyhow!("usage: new <title>"));
                }
                let note = note::create_note(&self.notebook, &title)?;
                self.message = format!("created {}", note.path.display());
                self.refresh();
            }
            "rename" => {
                let title = words.collect::<Vec<_>>().join(" ");
                if title.is_empty() {
                    return Err(anyhow!("usage: rename <title>"));
                }
                let selected = self
                    .selected_note()
                    .ok_or_else(|| anyhow!("no note selected"))?
                    .clone();
                let path = note::rename_note(&selected, &title, &self.notebook)?;
                self.message = format!("renamed to {}", path.display());
                self.refresh();
            }
            "move" => {
                let folder = PathBuf::from(words.collect::<Vec<_>>().join(" "));
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
            "delete" => {
                let selected = self
                    .selected_note()
                    .ok_or_else(|| anyhow!("no note selected"))?
                    .clone();
                note::delete_note(&selected)?;
                self.message = format!("deleted {}", selected.path.display());
                self.refresh();
            }
            "tag" => {
                let tags = words.map(str::to_string).collect::<Vec<_>>();
                let selected = self
                    .selected_note()
                    .ok_or_else(|| anyhow!("no note selected"))?
                    .clone();
                note::update_metadata(&selected.path, |meta| meta.tags = tags)?;
                self.message = "updated tags".to_string();
                self.refresh();
            }
            "type" => {
                let kind = words.collect::<Vec<_>>().join(" ");
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
                let links = words.map(str::to_string).collect::<Vec<_>>();
                let selected = self
                    .selected_note()
                    .ok_or_else(|| anyhow!("no note selected"))?
                    .clone();
                note::update_metadata(&selected.path, |meta| meta.links = links)?;
                self.message = "updated links".to_string();
                self.refresh();
            }
            "open" => {
                let selected = self
                    .selected_note()
                    .ok_or_else(|| anyhow!("no note selected"))?;
                open_external_editor(&selected.path).context("open external editor")?;
                self.message = "returned from editor".to_string();
                self.refresh();
            }
            "search" => {
                let query = words.collect::<Vec<_>>().join(" ");
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
                let message = words.collect::<Vec<_>>().join(" ");
                if message.is_empty() {
                    return Err(anyhow!("usage: commit <message>"));
                }
                git::commit(&self.notebook.root, &message)?;
                self.message = "committed changes".to_string();
                self.refresh();
            }
            "refresh" => {
                self.refresh();
                self.message = "refreshed".to_string();
            }
            "help" | "?" => {
                self.message = "Commands: new, rename, move, delete, tag, type, link, open, search, stage, unstage, stage-all, commit, refresh".to_string();
            }
            "quit" | "q" => self.should_quit = true,
            other => return Err(anyhow!("unknown command: {other}")),
        }

        Ok(())
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
