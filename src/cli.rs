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
}
