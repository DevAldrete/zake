use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotebookConfig {
    pub version: u16,
    pub notes_dir: PathBuf,
}

impl Default for NotebookConfig {
    fn default() -> Self {
        Self {
            version: 1,
            notes_dir: PathBuf::from("."),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notebook {
    pub root: PathBuf,
    pub config: NotebookConfig,
}

impl Notebook {
    pub fn init(path: impl AsRef<Path>) -> Result<Self> {
        let root = path
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| path.as_ref().to_path_buf());
        fs::create_dir_all(root.join(".zake/cache"))
            .with_context(|| format!("create {}", root.join(".zake/cache").display()))?;

        let config = NotebookConfig::default();
        fs::write(
            root.join(".zake/config.toml"),
            toml::to_string_pretty(&config)?,
        )
        .with_context(|| format!("write {}", root.join(".zake/config.toml").display()))?;

        if !root.join(".git").exists() {
            let status = Command::new("git")
                .arg("init")
                .current_dir(&root)
                .status()
                .context("run git init")?;
            if !status.success() {
                bail!("git init failed");
            }
        }

        Ok(Self { root, config })
    }

    pub fn discover(start: impl AsRef<Path>) -> Result<Self> {
        let start = start
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| start.as_ref().to_path_buf());
        for dir in start.ancestors() {
            let config_path = dir.join(".zake/config.toml");
            if config_path.exists() {
                return Self::load(dir);
            }
        }
        Err(anyhow!("not inside a Zake notebook; run `zake init` first"))
    }

    pub fn load(root: impl AsRef<Path>) -> Result<Self> {
        let root = root
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| root.as_ref().to_path_buf());
        let config_path = root.join(".zake/config.toml");
        let raw = fs::read_to_string(&config_path)
            .with_context(|| format!("read {}", config_path.display()))?;
        let config =
            toml::from_str(&raw).with_context(|| format!("parse {}", config_path.display()))?;
        Ok(Self { root, config })
    }

    pub fn notes_root(&self) -> PathBuf {
        self.root.join(&self.config.notes_dir)
    }

    pub fn validate(&self) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        if !self.root.join(".zake/config.toml").exists() {
            bail!("missing .zake/config.toml");
        }
        if !self.root.join(".git").exists() {
            warnings.push("notebook is not a Git repository".to_string());
        }
        if !self.notes_root().exists() {
            warnings.push(format!(
                "notes directory does not exist: {}",
                self.notes_root().display()
            ));
        }
        if Command::new("git").arg("--version").output().is_err() {
            warnings.push("git executable was not found".to_string());
        }
        if Command::new("rg").arg("--version").output().is_err() {
            warnings.push("rg executable was not found; search will be unavailable".to_string());
        }

        Ok(warnings)
    }
}
