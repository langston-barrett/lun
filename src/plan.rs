use std::{num::NonZeroUsize, sync::Arc};

use anyhow::Result;
use tracing::{debug, trace};

use crate::{cache, cmd, file, git, job, tool};

fn is_match(tool: &Arc<tool::Tool>, f: &file::File) -> bool {
    let path = f.path.as_path();
    if !tool.files.is_match(path) {
        return false;
    }
    if let Some(ignore) = &tool.ignore
        && ignore.is_match(path)
    {
        debug!("{}: ignored", f.path.display());
        return false;
    }
    trace!("{}: match", f.path.display());
    true
}

// The workings of this function are described in `doc/cache.md`.
fn need_file<C: cache::Cache + ?Sized>(
    cache: &mut C,
    git_refs: &[String],
    mtime_enabled: bool,
    tool: &Arc<tool::Tool>,
    file: &mut file::File,
) -> bool {
    let mtime_key = cache::Key::from_mtime(file, tool);
    if mtime_enabled && !cache.needed(&mtime_key) {
        debug!(
            "{}: not needed for {} (mtime)",
            file.path.display(),
            tool.display_name(),
        );
        return false;
    }
    if let Err(e) = file.fill_content_stamp() {
        debug!("{}: failed to read content ({e})", file.path.display());
        return false;
    }
    let content_key = cache::Key::from_content(file, tool);
    if !cache.needed(&content_key) {
        debug!(
            "{}: not needed for {} (content)",
            file.path.display(),
            tool.display_name(),
        );
        if mtime_enabled {
            cache.done(&mtime_key);
        }
        false
    } else if let Ok(true) = git::file_changed_from_refs(&file.path, git_refs) {
        true
    } else {
        cache.done(&content_key);
        if mtime_enabled {
            cache.done(&mtime_key);
        }
        false
    }
}

fn tool_commands<C: cache::Cache + ?Sized>(
    tool: &tool::Tool,
    files: &mut [file::File],
    cache: &mut C,
    git_refs: &[String],
    mtime_enabled: bool,
) -> Result<Option<cmd::Command>> {
    debug!("Planning for {}", tool.display_name());
    debug_assert!(!files.is_empty());
    let tool = Arc::new(tool.clone());

    let files = files
        .iter_mut()
        .filter_map(|f| {
            if is_match(&tool, f) && need_file(cache, git_refs, mtime_enabled, &tool, f) {
                Some(f.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if files.is_empty() {
        Ok(None)
    } else {
        Ok(Some(cmd::Command {
            tool: tool.clone(),
            files,
        }))
    }
}

pub(crate) fn plan<C: cache::Cache + ?Sized>(
    cache: &mut C,
    tools: &[tool::Tool],
    files: &[file::File],
    git_refs: &[String],
    cores: NonZeroUsize,
    no_batch: bool,
    mtime_enabled: bool,
) -> Result<Vec<cmd::Command>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    debug!("Collected {} files", files.len());
    let mut files = Vec::from(files);
    let mut commands = Vec::with_capacity(tools.len());
    for tool in tools {
        let Some(cmd) = tool_commands(tool, &mut files, cache, git_refs, mtime_enabled)? else {
            debug!(
                "No needed files for {}",
                tool.name.as_ref().unwrap_or(&tool.cmd)
            );
            continue;
        };
        debug_assert!(cmd.files.iter().all(|f| f.content_stamp.is_some()));
        commands.push(cmd);
    }
    Ok(job::create_jobs(commands, cores, no_batch))
}
