use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::{Result, invalid};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoBaseline {
    pub root: String,
    pub head: Option<String>,
    pub remote_refs: Vec<String>,
    pub dirty: Vec<String>,
}

pub fn toplevel(dir: &Path) -> Result<PathBuf> {
    git(dir, &["rev-parse", "--show-toplevel"]).map(PathBuf::from)
}

pub fn collect(dir: &Path) -> Result<RepoBaseline> {
    let root = toplevel(dir)?;
    let head = git(&root, &["rev-parse", "--verify", "--quiet", "HEAD"])
        .ok()
        .filter(|head| !head.is_empty());
    let remote_refs = lines(&git(
        &root,
        &[
            "for-each-ref",
            "--format=%(refname) %(objectname)",
            "refs/remotes",
        ],
    )?);
    let dirty = lines(&git(
        &root,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    )?)
    .into_iter()
    .filter(|line| {
        !line
            .get(3..)
            .is_some_and(|path| path.starts_with(".foxmorrow/"))
    })
    .collect();
    Ok(RepoBaseline {
        root: root.display().to_string(),
        head,
        remote_refs,
        dirty,
    })
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git").arg("-C").arg(dir).args(args).output()?;
    if !output.status.success() {
        return invalid(format!(
            "git {} failed in {}: {}",
            args.join(" "),
            dir.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string())
}

fn lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}
