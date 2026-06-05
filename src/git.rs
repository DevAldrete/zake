use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFileStatus {
    pub path: PathBuf,
    pub staged: Option<char>,
    pub unstaged: Option<char>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitStatus {
    pub files: Vec<GitFileStatus>,
}

impl GitStatus {
    pub fn staged(&self) -> Vec<&GitFileStatus> {
        self.files
            .iter()
            .filter(|file| file.staged.is_some() && !file.is_untracked())
            .collect()
    }

    pub fn unstaged(&self) -> Vec<&GitFileStatus> {
        self.files
            .iter()
            .filter(|file| file.unstaged.is_some())
            .collect()
    }

    pub fn untracked(&self) -> Vec<&GitFileStatus> {
        self.files
            .iter()
            .filter(|file| file.is_untracked())
            .collect()
    }
}

impl GitFileStatus {
    pub fn is_untracked(&self) -> bool {
        self.staged == Some('?') && self.unstaged == Some('?')
    }
}

pub fn status(root: &Path) -> Result<GitStatus> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "-z"])
        .current_dir(root)
        .output()
        .context("run git status")?;
    if !output.status.success() {
        bail!("git status failed");
    }
    Ok(parse_porcelain_z(&output.stdout))
}

pub fn stage(root: &Path, path: &Path) -> Result<()> {
    run_git(root, &["add", path.to_string_lossy().as_ref()])
}

pub fn stage_all(root: &Path) -> Result<()> {
    run_git(root, &["add", "."])
}

pub fn unstage(root: &Path, path: &Path) -> Result<()> {
    let path = path.to_string_lossy();
    run_git(root, &["restore", "--staged", path.as_ref()])
        .or_else(|_| run_git(root, &["rm", "--cached", "--", path.as_ref()]))
}

pub fn commit(root: &Path, message: &str) -> Result<()> {
    run_git(root, &["commit", "-m", message])
}

pub fn history(root: &Path, limit: usize) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["log", "--oneline", "--decorate", "-n", &limit.to_string()])
        .current_dir(root)
        .output()
        .context("run git log")?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect())
}

fn run_git(root: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .with_context(|| format!("run git {}", args.join(" ")))?;
    if !status.success() {
        bail!("git {} failed", args.join(" "));
    }
    Ok(())
}

pub fn parse_porcelain_z(raw: &[u8]) -> GitStatus {
    let mut files = Vec::new();
    let mut fields = raw
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());

    while let Some(field) = fields.next() {
        if field.len() < 4 {
            continue;
        }
        let staged = status_char(field[0]);
        let unstaged = status_char(field[1]);
        let path = String::from_utf8_lossy(&field[3..]).to_string();

        if matches!(staged, Some('R' | 'C')) {
            let _old_path = fields.next();
        }

        files.push(GitFileStatus {
            path: PathBuf::from(path),
            staged,
            unstaged,
        });
    }

    GitStatus { files }
}

fn status_char(byte: u8) -> Option<char> {
    match byte as char {
        ' ' => None,
        ch => Some(ch),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn parses_porcelain_z_status() {
        let status = parse_porcelain_z(b" M note.md\0?? new.md\0A  staged.md\0");
        assert_eq!(status.files.len(), 3);
        assert_eq!(status.unstaged()[0].path, PathBuf::from("note.md"));
        assert_eq!(status.untracked()[0].path, PathBuf::from("new.md"));
        assert_eq!(status.staged()[0].path, PathBuf::from("staged.md"));
    }

    #[test]
    fn stages_unstages_and_commits_in_temp_repo() {
        let dir = tempdir().unwrap();
        git_cmd(dir.path(), &["init"]);
        git_cmd(dir.path(), &["config", "user.email", "zake@example.test"]);
        git_cmd(dir.path(), &["config", "user.name", "Zake Test"]);
        fs::write(dir.path().join("note.md"), "hello").unwrap();

        let initial = status(dir.path()).unwrap();
        assert_eq!(initial.untracked()[0].path, PathBuf::from("note.md"));

        stage(dir.path(), Path::new("note.md")).unwrap();
        assert_eq!(
            status(dir.path()).unwrap().staged()[0].path,
            PathBuf::from("note.md")
        );

        unstage(dir.path(), Path::new("note.md")).unwrap();
        assert!(status(dir.path()).unwrap().staged().is_empty());

        stage(dir.path(), Path::new("note.md")).unwrap();
        commit(dir.path(), "add note").unwrap();
        assert!(status(dir.path()).unwrap().files.is_empty());
        assert!(history(dir.path(), 1).unwrap()[0].contains("add note"));
    }

    fn git_cmd(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap();
        assert!(status.success());
    }
}
