use std::path::Path;

use anyhow::{Context, Result};
use tracing::debug;

/// A git ref (branch, tag, or commit).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub(crate) struct Ref(pub String);

impl std::fmt::Display for Ref {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<String> for Ref {
    fn from(s: String) -> Self {
        Self(s)
    }
}

fn file_content_in_branch(path: &Path, branch: &Ref) -> Result<Option<Vec<u8>>> {
    let output = std::process::Command::new("git")
        .args(["show", &format!("{}:{}", branch, path.display())])
        .output()
        .with_context(|| format!("Failed to execute git show {}:{}", branch, path.display()))?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(output.stdout))
}

pub(crate) fn same_in_ref(path: &Path, content: &[u8], r: &Ref) -> bool {
    match file_content_in_branch(path, r) {
        Ok(Some(ref_content)) => {
            let matches = ref_content == content;
            if matches {
                debug!("{}: unchanged in ref {}", path.display(), r);
            }
            matches
        }
        Ok(None) => {
            debug!("{}: doesn't exist in ref {}", path.display(), r);
            false
        }
        Err(e) => {
            debug!(
                "{}: failed to check file in ref {} ({e})",
                path.display(),
                r,
            );
            false
        }
    }
}

/// Returns the subset of refs where all given files are unchanged.
pub(crate) fn unchanged_refs_all(paths: &[impl AsRef<Path>], refs: &[Ref]) -> Vec<Ref> {
    if paths.is_empty() {
        return refs.to_vec();
    }

    refs.iter()
        .filter(|r| {
            paths.iter().all(|path| {
                let path = path.as_ref();
                let Ok(content) = std::fs::read(path) else {
                    return false;
                };
                same_in_ref(path, &content, r)
            })
        })
        .cloned()
        .collect()
}

pub(crate) fn file_changed_from_refs(path: &Path, refs: &[Ref]) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }

    let current_content =
        std::fs::read(path).with_context(|| format!("Failed to read file: {}", path.display()))?;

    for r#ref in refs {
        match file_content_in_branch(path, r#ref) {
            Ok(Some(branch_content)) => {
                if branch_content == current_content {
                    debug!("{}: matches ref {}, skipping", path.display(), r#ref);
                    return Ok(false);
                }
            }
            Ok(None) => {
                debug!("{}: doesn't exist in ref {}", path.display(), r#ref);
            }
            Err(e) => {
                debug!(
                    "{}: failed to check file in ref {} ({e})",
                    path.display(),
                    r#ref,
                );
            }
        }
    }
    Ok(true)
}
