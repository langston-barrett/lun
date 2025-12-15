use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::debug;
use xxhash_rust::xxh3::Xxh3;

use crate::exec;
use crate::mtime::last_run_time;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Xxhash(pub(crate) u64);

/// Hash of file path, content, and metadata
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Stamp(pub(crate) Xxhash);

#[derive(Clone, Debug)]
pub(crate) struct File {
    pub(crate) path: PathBuf,
    pub(crate) size: usize,
    pub(crate) stamp: Stamp,
}

impl File {
    pub(crate) fn new(path: PathBuf) -> Result<Self> {
        let content =
            fs::read(&path).with_context(|| format!("Failed to read file: {}", path.display()))?;
        let metadata = fs::metadata(&path)
            .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;

        let mut hasher = Xxh3::new();
        hasher.update(path.as_os_str().as_encoded_bytes());
        hasher.update(&content);
        hasher.update(&metadata.len().to_le_bytes());
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            hasher.update(&metadata.uid().to_le_bytes());
            hasher.update(&metadata.gid().to_le_bytes());
            hasher.update(&metadata.mode().to_le_bytes());
        }

        Ok(Self {
            path,
            size: metadata.len() as usize,
            stamp: Stamp(Xxhash(hasher.digest())),
        })
    }
}

pub(crate) fn compute_hash(content: &[u8]) -> Xxhash {
    let mut hasher = Xxh3::new();
    hasher.update(content);
    Xxhash(hasher.digest())
}

pub(crate) fn collect_files(
    root: &Path,
    cache_dir: &Path,
    mtime_enabled: bool,
    progress_format: exec::ProgressFormat,
    config_path: &Path,
) -> Result<Vec<File>> {
    match progress_format {
        exec::ProgressFormat::No => (),
        exec::ProgressFormat::Yes => eprint!("\x1b[2K\r[0/?] Collecting files"),
        exec::ProgressFormat::Newline => eprintln!("\x1b[2K\r[0/?] Collecting files"),
    }
    drop(std::io::stderr().flush());
    let last_run = last_run_time(cache_dir, mtime_enabled, config_path)?;
    let mut files = Vec::new();
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
    for result in walker {
        let entry = result.with_context(|| "Failed to read directory entry")?;
        let path = entry.path();
        if path.is_dir()
            || !last_run.needed(path).with_context(|| {
                format!(
                    "Failed to check if {} was needed based on mtime",
                    path.display()
                )
            })?
        {
            continue;
        }

        debug!("Found {}", path.display());
        // This can fail due to TOCTTOU bugs between content/metadata
        if let Ok(file) = File::new(path.strip_prefix(root)?.to_path_buf()) {
            files.push(file);
        } else {
            debug!("Failed to process {}", path.display());
        }
    }

    // prevent very short-lived files (e.g., editor backups) from sneaking in
    files.retain(|f| f.path.exists());
    Ok(files)
}
