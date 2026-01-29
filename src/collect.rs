use std::{
    collections::HashSet,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context as _, Result};
use ignore::WalkBuilder;
use tracing::debug;

use crate::{
    file, filter,
    progress::{Format, Progress},
};

fn get_staged() -> Result<Vec<String>> {
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
    let files: Vec<_> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|p| {
            debug!("Found staged file {p}");
            p.to_string()
        })
        .collect();
    Ok(files)
}

pub(crate) fn walk(root: &Path, cache_dir: &Path) -> Result<Vec<PathBuf>> {
    let cache = fs::canonicalize(cache_dir).with_context(|| {
        format!(
            "Failed to canonicalize cache directory: {}",
            cache_dir.display()
        )
    })?;
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .filter_entry(move |e| {
            let path = e.path();

            path.extension().is_none_or(|e| e != "bck")
                && !path.starts_with("./.git")
                && !path.starts_with(".git")
                && fs::canonicalize(path).is_ok_and(|p| !p.starts_with(&cache))
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

pub(crate) fn go(
    root: &Path,
    cache_dir: &Path,
    progress_format: Format,
    out: &mut (impl Write + ?Sized),
    only: &[String],
    skip: &[String],
    staged: bool,
) -> Result<Vec<file::File>> {
    let mut progress = Progress::new(progress_format, None, out);
    progress.write("Collecting files");

    let mut only = only.to_vec();
    if staged {
        let s = HashSet::<String>::from_iter(get_staged()?);
        only.retain(|p| s.contains(p));
    }

    let mut paths = walk(root, cache_dir)?;
    filter::filter(&only, skip, &mut paths)?;

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
