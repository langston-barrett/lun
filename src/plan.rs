use std::io::Write;
use std::{num::NonZeroUsize, path::Path, sync::Arc};

use tracing::{debug, trace};

use crate::{cache, cmd, file as files, git, job, progress::Progress, tool};

fn is_match(tool: &Arc<tool::Tool>, f: &files::File, cwd: &Path) -> bool {
    let path = f.path.as_path();
    if !tool.files.is_match(path) {
        return false;
    }
    if let Some(ignore) = &tool.ignore {
        // Try matching against the path as-is, or stripped of cwd prefix for relative patterns
        let rel_path = path.strip_prefix(cwd).unwrap_or(path);
        if ignore.is_match(path) || ignore.is_match(rel_path) {
            debug!("{}: ignored", f.path.display());
            return false;
        }
    }
    trace!("{}: match", f.path.display());
    true
}

// The workings of this function are described in `doc/cache.md`.
fn need_file<C: cache::Cache + ?Sized>(
    cache: &mut C,
    refs: &[git::Ref],
    mtime_enabled: bool,
    tool: &Arc<tool::Tool>,
    file: &mut files::File,
) -> bool {
    let mtime_key = cache::Key::from_mtime(file, tool);
    if mtime_enabled && !tool.include_unchanged && !cache.needed(&mtime_key) {
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
    let is_needed = cache.needed(&content_key);

    if tool.include_unchanged {
        return true;
    }

    if !is_needed {
        debug!(
            "{}: not needed for {} (content)",
            file.path.display(),
            tool.display_name(),
        );
        if mtime_enabled {
            cache.done(&mtime_key);
        }
        false
    } else if let Ok(true) = git::file_changed_from_refs(&file.path, refs) {
        true
    } else {
        cache.done(&content_key);
        if mtime_enabled {
            cache.done(&mtime_key);
        }
        false
    }
}

fn tool_commands<C: cache::Cache + ?Sized, W: Write + ?Sized>(
    tool: &tool::Tool,
    files: &mut [files::File],
    cache: &mut C,
    refs: &[git::Ref],
    mtime_enabled: bool,
    cwd: &Path,
    progress: &mut Progress<'_, W>,
) -> Option<cmd::Command> {
    debug!("Planning for {}", tool.display_name());
    debug_assert!(!files.is_empty());
    let tool = Arc::new(tool.clone());
    let refs = git::unchanged_refs_all(&tool.configs, refs);
    let files = files
        .iter_mut()
        .filter_map(|f| {
            progress.increment();
            progress.write("Planning");
            if is_match(&tool, f, cwd) && need_file(cache, &refs, mtime_enabled, &tool, f) {
                Some(f.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if files.is_empty() {
        None
    } else {
        Some(cmd::Command {
            tool: tool.clone(),
            files,
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn plan<C: cache::Cache + ?Sized, W: Write + ?Sized>(
    cache: &mut C,
    tools: &[tool::Tool],
    files: &[files::File],
    refs: &[git::Ref],
    config_path: Option<&Path>,
    cwd: &Path,
    cores: NonZeroUsize,
    no_batch: bool,
    mtime_enabled: bool,
    progress: &mut Progress<'_, W>,
) -> Vec<cmd::Command> {
    debug!("Collected {} files", files.len());
    if files.is_empty() {
        return Vec::new();
    }
    let refs = match config_path {
        Some(path) => git::unchanged_refs_all(&[path], refs),
        None => refs.to_vec(),
    };
    debug!("{} refs with matching config file", refs.len());
    let mut files = Vec::from(files);
    let mut commands = Vec::with_capacity(tools.len());
    for tool in tools {
        let Some(cmd) = tool_commands(tool, &mut files, cache, &refs, mtime_enabled, cwd, progress)
        else {
            debug!(
                "No needed files for {}",
                tool.name.as_ref().unwrap_or(&tool.cmd)
            );
            continue;
        };
        debug_assert!(cmd.files.iter().all(|f| f.content_stamp.is_some()));
        commands.push(cmd);
    }
    job::create_jobs(commands, cores, no_batch)
}
