use std::{num::NonZeroUsize, sync::Arc};

use anyhow::Result;
use tracing::debug;

use crate::{cache, cmd, file, git, job, run::RunMode, tool};

fn tool_commands(
    tool: &tool::Tool,
    files: &[file::File],
    cache: &mut impl cache::Cache,
    mode: RunMode,
    git_refs: &[String],
) -> Result<Option<cmd::Command>> {
    debug!("Planning for {}", tool.display_name());
    debug_assert!(!files.is_empty());
    let tool = Arc::new(tool.clone());

    let matching_files = files
        .iter()
        .filter(|f| {
            let path = f.path.as_path();
            if !tool.files.is_match(path) {
                return false;
            }
            if let Some(ignore) = &tool.ignore
                && ignore.is_match(path)
            {
                debug!("Ignored: {}", f.path.display());
                return false;
            }
            debug!("Match: {}", f.path.display());
            true
        })
        .collect::<Vec<_>>();
    if matching_files.is_empty() {
        debug!("No files matching glob for {}", tool.display_name());
        return Ok(None);
    }

    let files_to_lint: Vec<&file::File> = matching_files
        .iter()
        .filter(|file| {
            let key = cache::Key::from_file_and_tool(file, &tool, mode);
            if cache.needed(&key) {
                if let Ok(true) = git::file_changed_from_refs(&file.path, git_refs) {
                    true
                } else {
                    cache.done(&key);
                    false
                }
            } else {
                debug!(
                    "Not needed for {}: {}",
                    tool.display_name(),
                    file.path.display()
                );
                false
            }
        })
        .copied()
        .collect();
    if files_to_lint.is_empty() {
        Ok(None)
    } else {
        let files = files_to_lint.iter().map(|f| (*f).clone()).collect();
        Ok(Some(cmd::Command {
            tool: tool.clone(),
            files,
        }))
    }
}

pub(crate) fn plan(
    cache: &mut impl cache::Cache,
    tools: &[tool::Tool],
    files: &[file::File],
    git_refs: &[String],
    cores: NonZeroUsize,
    mode: RunMode,
    no_batch: bool,
) -> Result<Vec<cmd::Command>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    debug!("Collected {} files", files.len());
    let mut commands = Vec::with_capacity(tools.len());
    for tool in tools {
        let Some(cmd) = tool_commands(tool, files, cache, mode, git_refs)? else {
            debug!(
                "No needed files for {}",
                tool.name.as_ref().unwrap_or(&tool.cmd)
            );
            continue;
        };
        commands.push(cmd);
    }
    Ok(job::create_jobs(commands, cores, no_batch))
}
