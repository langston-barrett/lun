use std::{fs, io::Write, path::Path, time};

use anyhow::{Context as _, Result};
use tracing::debug;
use xxhash_rust::xxh3::Xxh3;

use crate::file;

const LUN_VERSION: &str = env!("CARGO_PKG_VERSION");
const LAST_RUN: &str = "last-run";

#[derive(Debug)]
pub(crate) struct LastRun(Option<time::SystemTime>);

impl LastRun {
    pub(crate) fn needed(&self, path: &Path) -> Result<bool> {
        let Some(time) = self.0 else {
            return Ok(true);
        };
        let md = fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;
        if let Ok(file_mtime) = md.modified()
            && file_mtime <= time
        {
            debug!("Skipping {} (not modified since last run)", path.display());
            return Ok(false);
        }
        Ok(true)
    }
}

fn config_hash(config_path: &Path) -> Result<file::Xxhash> {
    let mut hasher = Xxh3::new();
    hasher.update(LUN_VERSION.as_bytes());
    if config_path.exists() {
        let config_content = fs::read(config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
        hasher.update(&config_content);
    }
    Ok(file::Xxhash(hasher.digest()))
}

pub(crate) fn last_run_time(
    cache_dir: &Path,
    mtime_enabled: bool,
    config_path: &Path,
) -> Result<LastRun> {
    if !mtime_enabled {
        return Ok(LastRun(None));
    }
    let last_run_path = cache_dir.join(LAST_RUN);
    if !last_run_path.exists() {
        return Ok(LastRun(None));
    }

    let current_hash = config_hash(config_path)?;
    let stored_hash_bytes = fs::read(&last_run_path)
        .with_context(|| format!("Failed to read last run hash: {}", last_run_path.display()))?;
    if stored_hash_bytes.len() != 8 {
        return Ok(LastRun(None));
    }

    let hash_array: [u8; 8] = stored_hash_bytes[..8]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to convert hash bytes to array"))?;
    let stored_hash = file::Xxhash(u64::from_le_bytes(hash_array));
    if stored_hash != current_hash {
        debug!("Version or config changed, ignoring mtimes");
        return Ok(LastRun(None));
    }

    let metadata = fs::metadata(&last_run_path)
        .with_context(|| format!("Failed to read last run time: {}", last_run_path.display()))?;
    let last_modified = metadata.modified()?;
    debug!("Last modified: {last_modified:?}");
    Ok(LastRun(Some(last_modified)))
}

pub(crate) fn update_last_run_time(cache_dir: &Path, config_path: &Path) -> Result<()> {
    let hash = config_hash(config_path)?;
    let last_run_path = cache_dir.join(LAST_RUN);
    let mut file = fs::File::create(&last_run_path).with_context(|| {
        format!(
            "Failed to create last run hash file: {}",
            last_run_path.display()
        )
    })?;
    file.write_all(&hash.0.to_le_bytes())
        .with_context(|| format!("Failed to write last run hash: {}", last_run_path.display()))?;
    Ok(())
}
