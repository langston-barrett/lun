use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::num::NonZeroUsize;
use std::os::unix::process::ExitStatusExt as _;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::{cmp, process, thread};

use anyhow::{Context, Result};
use rayon::prelude::*;
use tracing::{debug, error, trace};

use crate::cache;
use crate::cache::CacheWriter;
use crate::file;
use crate::progress::{Format, Progress};
use crate::run::{batch, cmd, report};

#[allow(clippy::too_many_arguments)]
pub(crate) fn exec(
    cache_writer: &mut (impl CacheWriter + ?Sized),
    batches: Vec<batch::Batch>,
    cores: NonZeroUsize,
    no_capture: bool,
    format: Format,
    color: bool,
    keep_going: bool,
    mtime_enabled: bool,
    start_time: Option<Instant>,
    out: &mut (impl Write + Send),
) -> Result<bool> {
    let n_batches = batches.len();
    let progress = Progress::new(format, Some(n_batches), None, color, out);
    if n_batches == 0 {
        progress.finalize("0 files linted", false);
        return Ok(true);
    }
    debug!(batches = n_batches, "Executing batches in parallel");
    let num_threads = cmp::min(cores.get(), n_batches);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .context("Failed to create rayon thread pool")?;

    let (tx, rx) = mpsc::channel::<report::Event>();

    let failed = AtomicBool::new(false);
    let remaining = AtomicUsize::new(n_batches);

    let total_files = batches
        .iter()
        .flat_map(|batch| batch.cmd.files.iter().map(|f| &f.path))
        .collect::<HashSet<_>>()
        .len();
    debug!("{total_files} unique files to lint");

    let (ok, all_hashes) = thread::scope(|s| -> Result<(bool, Vec<cache::KeyHash>)> {
        s.spawn(|| {
            report::reporter(keep_going, total_files, start_time, rx, progress);
        });

        let result = pool.install(|| -> Result<(bool, Vec<cache::KeyHash>)> {
            let tx = tx.clone();
            let results = batches
                .into_par_iter()
                .map(|batch| -> Result<(bool, Vec<cache::KeyHash>)> {
                    exec_batch(
                        no_capture,
                        keep_going,
                        mtime_enabled,
                        &failed,
                        &remaining,
                        &tx,
                        batch,
                    )
                })
                .collect::<Result<Vec<_>>>()?;

            let mut ok = true;
            let mut all_hashes = Vec::with_capacity(results.len());
            for (b, hashes) in results {
                ok &= b;
                all_hashes.extend(hashes);
            }
            Ok((ok, all_hashes))
        });

        // Close the channel to signal the reporter thread to finish
        drop(tx);
        // Reporter thread joins automatically when scope ends
        result
    })?;

    for hash in all_hashes {
        cache_writer.done_hash(hash);
    }

    Ok(ok)
}

fn exec_batch(
    no_capture: bool,
    keep_going: bool,
    mtime_enabled: bool,
    failed: &AtomicBool,
    remaining: &AtomicUsize,
    tx: &mpsc::Sender<report::Event>,
    batch: batch::Batch,
) -> std::result::Result<(bool, Vec<cache::KeyHash>), anyhow::Error> {
    if !keep_going && failed.load(Ordering::Relaxed) {
        return Ok((false, Vec::new()));
    }

    let c = batch.to_command();
    let cmd_str = batch::display_cmd(&c);
    debug!("running: {cmd_str}");
    let rep_batch = report::Batch::from(&batch);
    drop(tx.send(report::Event::Start {
        batch: rep_batch.clone(),
    }));
    let result = run(c, &cmd_str, no_capture, remaining, tx)?;
    let success = result.status.success();

    if !success {
        failed.store(true, Ordering::Relaxed);
        if let Some(output) = result.failure_output {
            drop(tx.send(report::Event::Failed { output }));
        }
    }
    debug!("{}: {cmd_str}", if success { "success" } else { "failed" });
    drop(tx.send(report::Event::Done {
        batch: rep_batch.clone(),
    }));
    remaining.fetch_sub(1, Ordering::Relaxed);
    let hashes = if success {
        done(batch.cmd, mtime_enabled)
    } else {
        Vec::new()
    };
    Ok((success, hashes))
}

struct RunResult {
    status: process::ExitStatus,
    /// Output to display on failure (only populated when capturing and command failed)
    failure_output: Option<Vec<u8>>,
}

fn run(
    mut c: process::Command,
    displayed_command: &str,
    no_capture: bool,
    remaining: &AtomicUsize,
    tx: &mpsc::Sender<report::Event>,
) -> Result<RunResult> {
    // TODO: This should depend on out_config
    // https://docs.astral.sh/ruff/faq/#how-can-i-disableforce-ruffs-color-output
    c.env("FORCE_COLOR", "1");
    // https://bixense.com/clicolors/
    c.env("CLICOLOR_FORCE", "1");
    // Avoid running on very short-lived files (e.g., editor backups)
    #[allow(clippy::unwrap_used)]
    if c.get_args().len() == 1 && !Path::new(c.get_args().next().unwrap()).exists() {
        return Ok(RunResult {
            status: process::ExitStatus::from_raw(0),
            failure_output: None,
        });
    }
    if no_capture {
        let status = c
            .status()
            .with_context(|| format!("Failed to execute command: {displayed_command}"))?;
        if !status.success() {
            error!("Command failed");
        }
        Ok(RunResult {
            status,
            failure_output: None,
        })
    } else {
        // Spawn process with piped stdout/stderr for streaming
        c.stdout(Stdio::piped());
        c.stderr(Stdio::piped());
        let mut child = c
            .spawn()
            .with_context(|| format!("Failed to execute command: {displayed_command}"))?;

        #[allow(clippy::unwrap_used)]
        let stdout = child.stdout.take().unwrap();
        #[allow(clippy::unwrap_used)]
        let stderr = child.stderr.take().unwrap();

        let start_time = Instant::now();
        let mut stdout_buffer = Vec::new();
        let mut stderr_buffer = Vec::new();

        // Create channels for reading stdout/stderr in separate threads
        let (output_tx, output_rx) = mpsc::channel::<(bool, Vec<u8>)>();

        // Spawn threads to read stdout and stderr
        thread::scope(|s| {
            let output_tx_clone = output_tx.clone();
            s.spawn(move || {
                let mut reader = BufReader::new(stdout);
                let mut line = Vec::new();
                loop {
                    line.clear();
                    match reader.read_until(b'\n', &mut line) {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            drop(output_tx_clone.send((true, line.clone())));
                        }
                        Err(_) => break,
                    }
                }
            });

            let output_tx_clone = output_tx.clone();
            s.spawn(move || {
                let mut reader = BufReader::new(stderr);
                let mut line = Vec::new();
                loop {
                    line.clear();
                    match reader.read_until(b'\n', &mut line) {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            drop(output_tx_clone.send((false, line.clone())));
                        }
                        Err(_) => break,
                    }
                }
            });

            drop(output_tx);

            // Collect output and stream lines when appropriate
            let mut last_output_time: Option<Instant> = None;
            while let Ok((is_stdout, line_bytes)) = output_rx.recv() {
                let buffer = if is_stdout {
                    &mut stdout_buffer
                } else {
                    &mut stderr_buffer
                };
                buffer.extend_from_slice(&line_bytes);

                // Check if we should stream this line
                let remaining = remaining.load(Ordering::Relaxed);
                trace!("remaining = {remaining}");
                let last = remaining == 1;
                if !last {
                    continue;
                }
                trace!("Last running: {}", batch::display_cmd(&c));

                let elapsed = start_time.elapsed();
                let slow = elapsed > Duration::from_secs(1);
                if !slow {
                    continue;
                }

                let rate_limit_ok = last_output_time
                    .is_none_or(|t| t.elapsed() >= crate::progress::TERMINAL_RATE_LIMIT);
                if rate_limit_ok {
                    let line_str = String::from_utf8_lossy(&line_bytes);
                    let line_str = line_str.trim_end();
                    if !line_str.is_empty() {
                        drop(tx.send(report::Event::Output {
                            line: line_str.to_string(),
                        }));
                        last_output_time = Some(Instant::now());
                    }
                }
            }
        });

        // Wait for the process to complete
        let status = child
            .wait()
            .with_context(|| format!("Failed to wait for command: {displayed_command}"))?;
        let success = status.success();

        if !stdout_buffer.is_empty() && success {
            trace!("{}", String::from_utf8_lossy(&stdout_buffer));
        }
        if !stderr_buffer.is_empty() && success {
            trace!("{}", String::from_utf8_lossy(&stderr_buffer));
        }

        let failure_output = if success {
            None
        } else {
            let mut buf = Vec::with_capacity(
                displayed_command.len() + stderr_buffer.len() + stdout_buffer.len() + 2,
            );
            buf.extend_from_slice(displayed_command.as_bytes());
            if !stdout_buffer.is_empty() {
                buf.extend_from_slice(b"\n");
                buf.extend_from_slice(stdout_buffer.trim_ascii_end());
            }
            if !stderr_buffer.is_empty() {
                buf.extend_from_slice(b"\n");
                buf.extend_from_slice(stderr_buffer.trim_ascii_end());
            }
            buf.push(b'\n');
            Some(buf)
        };
        Ok(RunResult {
            status,
            failure_output,
        })
    }
}

enum Reread {
    /// The tool modified the file; here's the fresh state.
    Modified(file::File),
    /// The mtime is unchanged; the tool didn't modify this file.
    Unchanged,
    /// We couldn't stat or read the file after the tool ran.
    Failed,
}

/// Re-read a file from disk to get its post-modification state.
///
/// Does a cheap stat first and compares mtime against the pre-run state.
/// Only reads file content if the mtime actually changed (i.e., the tool
/// modified the file).
fn reread_file(path: &Path, old_mtime: &file::Stamp) -> Reread {
    match file::File::new(path.to_path_buf()) {
        Ok(mut fresh) => {
            if fresh.mtime_stamp == *old_mtime {
                trace!("{}: mtime unchanged, skipping re-read", path.display());
                return Reread::Unchanged;
            }
            if let Err(e) = fresh.fill_content_stamp() {
                debug!("{}: failed to re-read after tool ran ({e})", path.display());
                Reread::Failed
            } else {
                Reread::Modified(fresh)
            }
        }
        Err(e) => {
            debug!("{}: failed to re-stat after tool ran ({e})", path.display());
            Reread::Failed
        }
    }
}

/// Compute cache keys for a file after a tool has run.
///
/// For tools that modify files (formatters, linters in fix mode), re-reads
/// from disk so we cache the post-fix state, not the stale pre-run state.
/// Skips the file (no keys pushed) if re-reading fails.
pub(super) fn cache_keys(
    out: &mut Vec<cache::KeyHash>,
    cmd_file: &file::File,
    tool: &crate::tool::Tool,
    mtime_enabled: bool,
) {
    let fresh;
    let f = if tool.modifies_files {
        match reread_file(&cmd_file.path, &cmd_file.mtime_stamp) {
            Reread::Modified(f) => {
                fresh = f;
                &fresh
            }
            Reread::Unchanged => cmd_file,
            Reread::Failed => return,
        }
    } else {
        debug_assert!(cmd_file.content_stamp.is_some()); // should happen in plan.rs
        cmd_file
    };
    let content_key = cache::Key::from_content(f, tool);
    out.push(cache::KeyHash::from(&content_key));
    if mtime_enabled {
        let mtime_key = cache::Key::from_mtime(f, tool);
        out.push(cache::KeyHash::from(&mtime_key));
    }
}

fn done(cmd: cmd::Command, mtime_enabled: bool) -> Vec<cache::KeyHash> {
    let tool = &cmd.tool;
    let mut hashes = Vec::with_capacity(if mtime_enabled {
        cmd.files.len() * 2
    } else {
        cmd.files.len()
    });
    for cmd_file in &cmd.files {
        cache_keys(&mut hashes, cmd_file, tool, mtime_enabled);
    }
    hashes
}
