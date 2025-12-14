use std::path::Path;

use anyhow::{Context, Result};
use tracing::debug;

fn file_content_in_branch(path: &Path, branch: &str) -> Result<Option<Vec<u8>>> {
    let output = std::process::Command::new("git")
        .args(["show", &format!("{branch}:{}", path.display())])
        .output()
        .with_context(|| format!("Failed to execute git show {branch}:{}", path.display()))?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(output.stdout))
}

pub(crate) fn file_changed_from_refs(path: &Path, refs: &[String]) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }

    let current_content =
        std::fs::read(path).with_context(|| format!("Failed to read file: {}", path.display()))?;

    for r#ref in refs {
        match file_content_in_branch(path, r#ref) {
            Ok(Some(branch_content)) => {
                if branch_content == current_content {
                    debug!("{} matches ref {}, skipping", path.display(), r#ref);
                    return Ok(false);
                }
            }
            Ok(None) => {
                debug!("{} doesn't exist in ref {}", path.display(), r#ref);
            }
            Err(e) => {
                debug!(
                    "Failed to check file {} in ref {}: {}",
                    path.display(),
                    r#ref,
                    e
                );
            }
        }
    }
    Ok(true)
}
