use std::path::PathBuf;

use anyhow::{Context as _, Result};
use globset::{Glob, GlobMatcher};
use tracing::debug;

fn only_matchers(only_patterns: &[String]) -> Result<Vec<GlobMatcher>> {
    let only = only_patterns
        .iter()
        .map(|pattern| {
            Glob::new(pattern)
                .with_context(|| format!("Invalid `only` glob pattern: {pattern}"))
                .map(|g| g.compile_matcher())
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(only)
}

fn skip_matchers(skip_patterns: &[String]) -> Result<Vec<GlobMatcher>> {
    let skip = skip_patterns
        .iter()
        .map(|pattern| {
            Glob::new(pattern)
                .with_context(|| format!("Invalid `skip` glob pattern: {pattern}"))
                .map(|g| g.compile_matcher())
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(skip)
}

pub(crate) fn filter(only: &[String], skip: &[String], paths: &mut Vec<PathBuf>) -> Result<()> {
    let only = only_matchers(only)?;
    let skip = skip_matchers(skip)?;
    if only.is_empty() && skip.is_empty() {
        return Ok(());
    }
    paths.retain(|p| {
        if !only.is_empty() && !only.iter().any(|m| m.is_match(p)) {
            debug!("File ignored due to `--only`: {}", p.display());
            return false;
        }
        if skip.iter().any(|m| m.is_match(p)) {
            debug!("File ignored due to `--skip`: {}", p.display());
            return false;
        }
        true
    });
    Ok(())
}
