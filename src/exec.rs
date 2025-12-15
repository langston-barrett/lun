use std::collections::HashSet;
use std::io::Write as _;
use std::num::NonZeroUsize;
use std::os::unix::process::ExitStatusExt as _;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::{cmp, io, process, thread};

use anyhow::{Context, Result};
use rayon::prelude::*;
use tracing::{debug, error, trace};

use crate::cache::CacheWriter;
use crate::job;
use crate::{cache, cmd};

#[derive(Debug)]
enum ReporterEvent {
    Start { cmd: String },
    Done { cmd: String },
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ProgressFormat {
    No,
    Yes,
    Newline,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn exec(
    cache_writer: &mut impl CacheWriter,
    batches: Vec<cmd::Command>,
    cores: NonZeroUsize,
    no_capture: bool,
    format: ProgressFormat,
    keep_going: bool,
    mtime_enabled: bool,
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

    let (tx, rx) = mpsc::channel::<ReporterEvent>();
    let reporter_handle = thread::spawn(move || reporter(num_threads, n_batches, rx, format));

    let failed = AtomicBool::new(false);

    let (ok, all_hashes) = pool.install(|| -> Result<(bool, Vec<cache::KeyHash>)> {
        let tx = tx.clone();
        let results = batches
            .into_par_iter()
            .map(|cmd| -> Result<(bool, Vec<cache::KeyHash>)> {
                if !keep_going && failed.load(Ordering::Relaxed) {
                    return Ok((false, Vec::new()));
                }

                let c = cmd.to_command();
                let cmd_str = job::display_cmd(&c);
                debug!("Running {}", cmd_str);
                tx.send(ReporterEvent::Start {
                    cmd: cmd_str.clone(),
                })
                .ok();
                let success = run(c, &cmd_str, no_capture)?.success();

                if !success {
                    failed.store(true, Ordering::Relaxed);
                }
                debug!(
                    "Finished {} ({})",
                    cmd_str,
                    if success { "success" } else { "failed" },
                );
                tx.send(ReporterEvent::Done { cmd: cmd_str }).ok();
                let hashes = if success {
                    done(cmd, mtime_enabled)?
                } else {
                    Vec::new()
                };
                Ok((success, hashes))
            })
            .collect::<Result<Vec<_>>>()?;

        let mut ok = true;
        let mut all_hashes = Vec::with_capacity(results.len());
        for (b, hashes) in results {
            ok &= b;
            all_hashes.extend(hashes.into_iter());
        }
        Ok((ok, all_hashes))
    })?;

    // Close the channel to signal the reporter thread to finish
    drop(tx);
    #[allow(clippy::expect_used)]
    reporter_handle.join().expect("Reporter thread panicked");

    for hash in all_hashes {
        cache_writer.done_hash(hash);
    }

    Ok(ok)
}

fn reporter(
    n_threads: usize,
    n_batches: usize,
    rx: mpsc::Receiver<ReporterEvent>,
    format: ProgressFormat,
) {
    let mut running = HashSet::with_capacity(n_threads);
    let mut completed = 0;
    let mut current_cmd: Option<String> = None;
    let total = n_batches;

    loop {
        match rx.recv() {
            Ok(ReporterEvent::Start { cmd }) => {
                running.insert(cmd.clone());
                if current_cmd.is_none() {
                    report(format, completed + 1, total, &cmd);
                    current_cmd = Some(cmd);
                }
            }
            Ok(ReporterEvent::Done { cmd }) => {
                running.remove(&cmd);
                completed += 1;

                if current_cmd.as_ref() == Some(&cmd) {
                    current_cmd = running.iter().next().cloned();
                }

                if let Some(current) = &current_cmd {
                    report(format, completed + 1, total, current);
                } else if completed < total {
                    report(format, completed + 1, total, "");
                }
            }
            Err(_) => {
                // nb: final newline printing happens in `run::report_result`
                break;
            }
        }
    }
}

fn report(format: ProgressFormat, completed: usize, total: usize, cmd: &str) {
    if cmd.is_empty() {
        match format {
            ProgressFormat::No => (),
            ProgressFormat::Yes => eprint!("\x1b[2K\r[{completed}/{total}]"),
            ProgressFormat::Newline => eprintln!("\x1b[2K\r[{completed}/{total}]"),
        }
    } else {
        let shorter = &cmd[0..cmp::min(60, cmd.len())];
        match format {
            ProgressFormat::No => (),
            ProgressFormat::Yes => eprint!("\x1b[2K\r[{completed}/{total}] {shorter}"),
            ProgressFormat::Newline => eprintln!("\x1b[2K\r[{completed}/{total}] {shorter}"),
        };
    }
    drop(io::stderr().flush());
}

fn run(
    mut c: process::Command,
    displayed_command: &str,
    no_capture: bool,
) -> Result<process::ExitStatus> {
    // https://docs.astral.sh/ruff/faq/#how-can-i-disableforce-ruffs-color-output
    c.env("FORCE_COLOR", "1");
    // https://bixense.com/clicolors/
    c.env("CLICOLOR_FORCE", "1");
    // Avoid running on very short-lived files (e.g., editor backups)
    #[allow(clippy::unwrap_used)]
    if c.get_args().len() == 1 && !Path::new(c.get_args().next().unwrap()).exists() {
        return Ok(process::ExitStatus::from_raw(0));
    }
    if no_capture {
        let status = c
            .status()
            .with_context(|| format!("Failed to execute command: {displayed_command}"))?;
        if !status.success() {
            error!("Command failed");
        }
        Ok(status)
    } else {
        let out = c
            .output()
            .with_context(|| format!("Failed to execute command: {displayed_command}"))?;
        let success = out.status.success();
        if !out.stdout.is_empty() && success {
            trace!("{}", String::from_utf8_lossy(&out.stdout));
        }
        if !out.stderr.is_empty() && success {
            trace!("{}", String::from_utf8_lossy(&out.stderr));
        }
        if !success {
            io::stdout().write_all(b"\n")?;
            io::stdout().write_all(out.stdout.as_slice())?;
            io::stdout().write_all(b"\n")?;
            io::stderr().write_all(out.stderr.as_slice())?;
        }
        Ok(out.status)
    }
}

fn done(cmd: cmd::Command, mtime_enabled: bool) -> Result<Vec<cache::KeyHash>> {
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
    Ok(hashes)
}
