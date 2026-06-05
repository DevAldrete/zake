use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "zake", version, about = "Fast Git-backed note management TUI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialize a Zake notebook.
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Validate notebook config, Git, ripgrep, and note index health.
    Doctor {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Create a Markdown note stub with YAML frontmatter.
    New {
        title: String,
        #[arg(long = "type")]
        kind: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long = "link")]
        links: Vec<String>,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// List notes, optionally filtered by tag or type.
    List {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long = "type")]
        kind: Option<String>,
    },
    /// Show note metadata, links, backlinks, and path.
    Show {
        note: String,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Rename a note by title or path.
    Rename {
        note: String,
        title: String,
        #[arg(long)]
        update_links: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Move a note into a folder under the notebook notes directory.
    Move {
        note: String,
        folder: PathBuf,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Delete a note by title or path.
    Delete {
        note: String,
        #[arg(long)]
        yes: bool,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// List links from one note and whether they resolve.
    Links {
        note: String,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// List notes linking to one note.
    Backlinks {
        note: String,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// List unresolved internal links.
    Broken {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// List notes with no internal links or backlinks.
    Orphans {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Replace note metadata fields.
    Set {
        note: String,
        #[arg(long = "type")]
        kind: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long = "link")]
        links: Vec<String>,
        #[arg(long)]
        clear_tags: bool,
        #[arg(long)]
        clear_links: bool,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Open a note in $EDITOR.
    Open {
        note: String,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Search the notebook with ripgrep.
    Search {
        query: String,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Show notebook Git status.
    Status {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Stage a path in the notebook Git repo.
    Stage {
        target: PathBuf,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Unstage a path in the notebook Git repo.
    Unstage {
        target: PathBuf,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Stage all notebook changes.
    StageAll {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Commit staged notebook changes.
    Commit {
        message: String,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Show recent notebook Git history.
    History {
        #[arg(short = 'n', long, default_value_t = 10)]
        limit: usize,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Show Git diff for the notebook or one path.
    Diff {
        target: Option<PathBuf>,
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },
    /// Stage all notebook changes and commit them.
    Snapshot {
        message: String,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Create or show today's daily note.
    Today {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Create or show this week's weekly note.
    Week {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}
