#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::unwrap_used)]

use std::{collections::HashSet, fmt::Write, fs, num::NonZeroUsize, path::Path, process};

use anyhow::{Context as _, Result};
use tracing::{debug, error, trace};
use xxhash_rust::xxh3::Xxh3;

use crate::{cache, cache::CacheWriter, cmd};

#[allow(clippy::too_many_arguments)]
pub(crate) fn exec(
    cache: &mut (impl CacheWriter + ?Sized),
    cache_dir: &Path,
    batches: Vec<cmd::Command>,
    cores: NonZeroUsize,
    dry_run: bool,
    no_capture: bool,
    keep_going: bool,
    mtime_enabled: bool,
) -> Result<bool> {
    let ninja_file = cache_dir.join("build.ninja");
    if batches.is_empty() {
        drop(fs::remove_file(&ninja_file));
        return Ok(true);
    }

    generate_ninja_file(cache_dir, &ninja_file, &batches)?;
    if dry_run {
        return Ok(true);
    }

    let mut cmd = process::Command::new("ninja");
    cmd.arg("-f").arg(&ninja_file);
    cmd.arg("-j").arg(cores.get().to_string());
    if keep_going {
        cmd.args(["-k", "0"]);
    }
    debug!("Running ninja -f {}", ninja_file.display());

    let builddir = cache_dir.join("ninja");
    fs::create_dir_all(&builddir)
        .with_context(|| format!("Failed to create builddir: {}", builddir.display()))?;

    let executed_targets = if no_capture {
        let status = cmd
            .status()
            .context("Failed to execute ninja. Is ninja installed?")?;
        if !status.success() {
            return Ok(false);
        }
        // When not capturing output, we can't parse which targets were executed,
        // so mark all targets as executed if ninja succeeded
        batches.iter().map(tgt_name).collect()
    } else {
        let out = cmd
            .output()
            .context("Failed to execute ninja. Is ninja installed?")?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        trace!("{stdout}");
        trace!("{stderr}");
        if !out.status.success() {
            error!("{stdout}\n{stderr}");
            return Ok(false);
        }
        parse_ninja_output(&stdout, &stderr, &batches, &builddir)
    };

    for cmd in batches {
        let target_name = tgt_name(&cmd);
        if executed_targets.contains(&target_name) {
            let tool = cmd.tool.clone();
            for file in &cmd.files {
                debug_assert!(file.content_stamp.is_some()); // in plan.rs
                let content_key = cache::Key::from_content(file, &tool);
                cache.done(&content_key);
                if mtime_enabled {
                    let mtime_key = cache::Key::from_mtime(file, &tool);
                    cache.done(&mtime_key);
                }
            }
        }
    }

    Ok(true)
}

fn tgt_name(cmd: &cmd::Command) -> String {
    let hash = cmd_hash(cmd);
    format!("$builddir/{hash:032x}")
}

fn cmd_hash(cmd: &cmd::Command) -> u128 {
    let mut hasher = Xxh3::new();
    let cmd_obj = cmd.to_command();
    let program_str = cmd_obj.get_program().to_string_lossy();
    hasher.update(program_str.as_bytes());
    hasher.update(&[0]);
    for arg in cmd_obj.get_args() {
        let arg_str = arg.to_string_lossy();
        hasher.update(arg_str.as_bytes());
        hasher.update(&[0]);
    }
    for file in &cmd.files {
        let path_str = file.path.to_string_lossy();
        hasher.update(path_str.as_bytes());
        hasher.update(&[0]);
    }
    hasher.digest128()
}

fn generate_ninja_file(
    cache_dir: &Path,
    ninja_file: &Path,
    batches: &[cmd::Command],
) -> Result<()> {
    debug!("Generating {}", ninja_file.display());
    let builddir = cache_dir.join("ninja");
    let mut content = format!("builddir={}\n\n", builddir.display());
    content.push_str("rule run\n");
    content.push_str("  command = $cmd && touch $out\n");
    content.push_str("  description = Running $desc\n\n");
    content.reserve(batches.len()); // at least

    for cmd in batches {
        let cmd_obj = cmd.to_command();
        let mut cmd_parts = Vec::new();
        let program_str = cmd_obj.get_program().to_string_lossy();
        cmd_parts.push(escape_ninja_string(&program_str));
        for arg in cmd_obj.get_args() {
            let arg_str = arg.to_string_lossy();
            if arg_str.contains(' ') || arg_str.contains('$') || arg_str.contains(':') {
                cmd_parts.push(format!(
                    "\"{}\"",
                    escape_ninja_string(&arg_str).replace('"', "\\\"")
                ));
            } else {
                cmd_parts.push(escape_ninja_string(&arg_str));
            }
        }
        let cmd_str = cmd_parts.join(" ");

        let desc = describe(&cmd_obj);
        let name = tgt_name(cmd);
        writeln!(content, "build {name}: run",).unwrap();
        writeln!(content, "  cmd = {}", escape_ninja_string(&cmd_str)).unwrap();
        writeln!(content, "  desc = {}", escape_ninja_string(&desc)).unwrap();
        writeln!(content).unwrap();
    }

    fs::write(ninja_file, content)
        .with_context(|| format!("Failed to write Ninja file: {}", ninja_file.display()))?;
    Ok(())
}

fn describe(cmd: &process::Command) -> String {
    format!(
        "{} {}",
        cmd.get_program().display(),
        cmd.get_args()
            .map(|a| a.display().to_string())
            .collect::<Vec<_>>()
            .join(" ")
    )
}

fn parse_ninja_output(
    stdout: &str,
    stderr: &str,
    batches: &[cmd::Command],
    builddir: &Path,
) -> HashSet<String> {
    let mut executed = HashSet::new();

    let mut desc_to_target: Vec<(String, String)> = Vec::new();
    for cmd in batches {
        let target_name = tgt_name(cmd);
        let desc = describe(&cmd.to_command());
        desc_to_target.push((desc, target_name));
    }

    // Parse stdout for lines like "[1/5] Running $desc"
    // These indicate targets that were actually executed (not up-to-date)
    // Ninja outputs "Running $desc" when executing a target
    for line in stdout.lines() {
        if line.contains("Running") && line.starts_with('[') {
            // Extract the description part after "Running "
            if let Some(running_pos) = line.find("Running ") {
                let desc_part = &line[running_pos + 8..].trim(); // Skip "Running "
                // Match this description to a target by comparing normalized strings
                for (desc, target_name) in &desc_to_target {
                    // Normalize whitespace for comparison
                    let normalized_desc = desc.split_whitespace().collect::<Vec<_>>().join(" ");
                    let normalized_part =
                        desc_part.split_whitespace().collect::<Vec<_>>().join(" ");
                    // Check if descriptions match (either exact or one contains the other)
                    if normalized_part == normalized_desc
                        || normalized_part.starts_with(&normalized_desc)
                        || normalized_desc.starts_with(&normalized_part)
                    {
                        executed.insert(target_name.clone());
                        break;
                    }
                }
            }
        }
    }

    for line in stderr.lines() {
        if let Some(target) = extract_target_from_failed_line(line, builddir) {
            executed.remove(&target);
        }
    }

    executed
}

fn extract_target_from_failed_line(line: &str, builddir: &Path) -> Option<String> {
    if let Some(line) = line.strip_prefix("FAILED: ") {
        let target = line.trim();
        // Check if this target is in our builddir
        // Ninja outputs the full path, so we check if it starts with builddir
        if let Some(builddir_str) = builddir.to_str()
            && target.starts_with(builddir_str)
        {
            return Some(target.to_string());
        }
        // Also handle relative paths (Ninja might output relative to builddir)
        if let Some(builddir_name) = builddir.file_name().and_then(|n| n.to_str())
            && target.starts_with(builddir_name)
        {
            // Reconstruct full path
            let relative_part = target
                .strip_prefix(builddir_name)
                .and_then(|s| s.strip_prefix('/'))
                .unwrap_or(target);
            return Some(builddir.join(relative_part).display().to_string());
        }
    }
    None
}

fn escape_ninja_string(s: &str) -> String {
    // Escape special Ninja characters: $ : | \n
    // Note: spaces don't need escaping in command lines
    s.replace('$', "$$")
        .replace(':', "$:")
        .replace('|', "$|")
        .replace('\n', "$\n")
}
