use std::io::Write;
use std::num::NonZeroUsize;
use std::os::unix::process::ExitStatusExt as _;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::{cmp, process, thread};

use anyhow::{Context, Result};
use rayon::prelude::*;
use tracing::{debug, error, trace};

use crate::cache;
use crate::cache::CacheWriter;
use crate::progress::{Format, Progress};
use crate::run::{batch, cmd, report};

#[allow(clippy::too_many_arguments)]
pub(crate) fn exec(
    cache_writer: &mut (impl CacheWriter + ?Sized),
    batches: Vec<batch::Batch>,
    cores: NonZeroUsize,
    no_capture: bool,
    format: Format,
    keep_going: bool,
    mtime_enabled: bool,
    out: &mut (impl Write + Send),
) -> Result<bool> {
    if batches.is_empty() {
        return Ok(true);
    }
    let n_batches = batches.len();
    debug!(batches = n_batches, "Executing batches in parallel");
    let num_threads = cmp::min(cores.get(), n_batches);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .context("Failed to create rayon thread pool")?;

    let (tx, rx) = mpsc::channel::<report::Event>();

    let failed = AtomicBool::new(false);

    let (ok, all_hashes) = thread::scope(|s| -> Result<(bool, Vec<cache::KeyHash>)> {
        s.spawn(|| {
            report::reporter(
                keep_going,
                num_threads,
                rx,
                Progress::new(format, Some(n_batches), out),
            );
        });

        let result = pool.install(|| -> Result<(bool, Vec<cache::KeyHash>)> {
            let tx = tx.clone();
            let results = batches
                .into_par_iter()
                .map(|batch| -> Result<(bool, Vec<cache::KeyHash>)> {
                    exec_batch(no_capture, keep_going, mtime_enabled, &failed, &tx, batch)
                })
                .collect::<Result<Vec<_>>>()?;

            let mut ok = true;
            let mut all_hashes = Vec::with_capacity(results.len());
            for (b, hashes) in results {
                ok &= b;
                all_hashes.extend(hashes.into_iter());
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
    let result = run(c, &cmd_str, no_capture)?;
    let success = result.status.success();

    if !success {
        failed.store(true, Ordering::Relaxed);
        if let Some(output) = result.failure_output {
            drop(tx.send(report::Event::Failed { output }));
        }
    }
    debug!("{}: {cmd_str}", if success { "success" } else { "failed" });
    drop(tx.send(report::Event::Done { batch: rep_batch }));
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

fn run(mut c: process::Command, displayed_command: &str, no_capture: bool) -> Result<RunResult> {
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
        let output = c
            .output()
            .with_context(|| format!("Failed to execute command: {displayed_command}"))?;
        let success = output.status.success();
        if !output.stdout.is_empty() && success {
            trace!("{}", String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() && success {
            trace!("{}", String::from_utf8_lossy(&output.stderr));
        }
        let failure_output = if success {
            None
        } else {
            let mut buf = Vec::new();
            buf.extend_from_slice(b"\n");
            buf.extend_from_slice(displayed_command.as_bytes());
            buf.extend_from_slice(b"\n");
            buf.extend_from_slice(&output.stdout);
            buf.extend_from_slice(b"\n");
            buf.extend_from_slice(&output.stderr);
            Some(buf)
        };
        Ok(RunResult {
            status: output.status,
            failure_output,
        })
    }
}

fn done(cmd: cmd::Command, mtime_enabled: bool) -> Vec<cache::KeyHash> {
    let tool = cmd.tool.clone();
    let mut hashes = Vec::with_capacity(if mtime_enabled {
        cmd.files.len() * 2
    } else {
        cmd.files.len()
    });
    for file in &cmd.files {
        debug_assert!(file.content_stamp.is_some()); // should happen in plan.rs
        let content_key = cache::Key::from_content(file, &tool);
        hashes.push(cache::KeyHash::from(&content_key));
        if mtime_enabled {
            let mtime_key = cache::Key::from_mtime(file, &tool);
            hashes.push(cache::KeyHash::from(&mtime_key));
        }
    }
    hashes
}
