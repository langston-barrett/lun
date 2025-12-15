use std::{path::PathBuf, process};

use anyhow::{Context as _, Result};
use tracing::debug;

use crate::file;

fn get_staged_files() -> Result<Vec<PathBuf>> {
    let output = process::Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACMR"])
        .output()
        .with_context(|| "Failed to execute git diff --cached")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "git diff --cached failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<PathBuf> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|p| {
            debug!("Found staged file {p}");
            PathBuf::from(p)
        })
        .collect();
    Ok(files)
}

pub(crate) fn collect_staged_files() -> Result<Vec<file::File>> {
    let staged_paths = get_staged_files()?;
    let mut files = Vec::new();
    let root = PathBuf::from(".");
    for path in staged_paths {
        let full_path = root.join(&path);
        if !full_path.exists() {
            continue;
        }
        files.push(file::File::new(path)?);
    }
    Ok(files)
}
