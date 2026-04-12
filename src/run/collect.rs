use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context as _, Result};
use ignore::WalkBuilder;
use tracing::debug;

use crate::{
    file,
    progress::{Format, Progress},
    run::filter,
};

fn get_staged(root: &Path) -> Result<Vec<PathBuf>> {
    let output = process::Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACMR"])
        .current_dir(root)
        .output()
        .with_context(|| "Failed to execute git diff --cached")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "git diff --cached failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let strip = root == env::current_dir()?;
    let files: Vec<_> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|p| {
            debug!("Found staged file {p}");
            if strip {
                PathBuf::from(p)
            } else {
                root.join(p)
            }
        })
        .collect();
    Ok(files)
}

fn get_vcs_files(root: &Path) -> Result<Vec<PathBuf>> {
    let output = process::Command::new("git")
        .args(["ls-files", "--exclude-standard"])
        .current_dir(root)
        .output()
        .with_context(|| "Failed to execute git ls-files")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let strip = root == env::current_dir()?;
    let files: Vec<_> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|p| {
            debug!("Found VCS-tracked file {p}");
            if strip {
                PathBuf::from(p)
            } else {
                root.join(p)
            }
        })
        .collect();
    Ok(files)
}

pub(crate) fn walk(root: &Path, cache_dir: &Path) -> Result<Vec<PathBuf>> {
    // Canonicalization may fail if the cache directory doesn't exist yet
    // (e.g., when --no-cache is used), in which case we skip cache filtering.
    let cache = fs::canonicalize(cache_dir).ok();
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .filter_entry(move |e| {
            let path = e.path();

            // Check if any component is .git
            let has_git_dir = path.components().any(|c| c.as_os_str() == ".git");

            path.extension().is_none_or(|e| e != "bck")
                && !has_git_dir
                && cache
                    .as_ref()
                    .is_none_or(|cache| fs::canonicalize(path).is_ok_and(|p| !p.starts_with(cache)))
        })
        .build();
    let mut paths = Vec::new();
    let strip = root == env::current_dir()?;
    for result in walker {
        let entry = result.with_context(|| "Failed to read directory entry")?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let path = if strip {
            path.strip_prefix(root)?
        } else {
            path
        };
        debug!("Found {}", path.display());
        paths.push(path.to_path_buf());
    }
    Ok(paths)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn go(
    root: &Path,
    cache_dir: &Path,
    progress_format: Format,
    color: bool,
    out: &mut (impl Write + ?Sized),
    only: &[String],
    skip: &[String],
    staged: bool,
    vcs: bool,
) -> Result<Vec<file::File>> {
    let mut progress = Progress::new(progress_format, None, None, color, out);
    progress.write("Collecting files");
    progress.done();

    let mut paths = if staged {
        get_staged(root)?
    } else if vcs {
        get_vcs_files(root)?
    } else {
        walk(root, cache_dir)?
    };
    filter::filter(only, skip, &mut paths)?;
    #[cfg(test)]
    paths.sort_unstable();

    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        match file::File::new(path) {
            Ok(file) => {
                files.push(file);
            }
            // This can fail due to TOCTTOU bugs between content/metadata
            Err(e) => debug!("{e}"),
        }
    }
    // prevent very short-lived files (e.g., editor backups) from sneaking in
    files.retain(|f| f.path.exists());

    Ok(files)
}
