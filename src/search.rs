use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub path: PathBuf,
    pub line: usize,
    pub text: String,
}

pub fn ripgrep(root: &Path, query: &str) -> Result<Vec<SearchHit>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let output = Command::new("rg")
        .args(["--line-number", "--no-heading", "--color", "never", query])
        .current_dir(root)
        .output()
        .context("run rg")?;

    if !output.status.success() && output.status.code() != Some(1) {
        bail!("rg failed");
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_rg_line)
        .collect())
}

fn parse_rg_line(line: &str) -> Option<SearchHit> {
    let mut parts = line.splitn(3, ':');
    let path = parts.next()?;
    let line_no = parts.next()?.parse().ok()?;
    let text = parts.next()?.to_string();
    Some(SearchHit {
        path: PathBuf::from(path),
        line: line_no,
        text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rg_line() {
        let hit = parse_rg_line("notes/a.md:12:hello: world").unwrap();
        assert_eq!(hit.path, PathBuf::from("notes/a.md"));
        assert_eq!(hit.line, 12);
        assert_eq!(hit.text, "hello: world");
    }
}
