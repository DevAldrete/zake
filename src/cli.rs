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
