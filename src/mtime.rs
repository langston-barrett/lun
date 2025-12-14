use std::{fs, path::Path, time};

use anyhow::{Context as _, Result};
use tracing::debug;

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

pub(crate) fn last_run_time(cache_dir: &Path, mtime_enabled: bool) -> Result<LastRun> {
    if !mtime_enabled {
        return Ok(LastRun(None));
    }
    let last_run_path = cache_dir.join(LAST_RUN);
    if !last_run_path.exists() {
        return Ok(LastRun(None));
    }
    let metadata = fs::metadata(&last_run_path)
        .with_context(|| format!("Failed to read last run time: {}", last_run_path.display()))?;
    let last_modified = metadata.modified()?;
    debug!("Last modified: {last_modified:?}");
    Ok(LastRun(Some(last_modified)))
}

pub(crate) fn update_last_run_time(cache_dir: &Path) -> Result<()> {
    let last_run_path = cache_dir.join(LAST_RUN);
    fs::File::create(&last_run_path).with_context(|| {
        format!(
            "Failed to create last run time file: {}",
            last_run_path.display()
        )
    })?;
    Ok(())
}
