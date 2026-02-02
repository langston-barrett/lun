//! Format:
//!
//! ```text
//! [5/32] clippy (1/1), ttlint (2-3/8), ruff (6/10)...
//! ```

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    io::Write,
    sync::{Arc, mpsc},
    time::Instant,
};

use tracing::error;

use crate::{progress::Progress, run::batch};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct Batch {
    tot: usize,
    name: Arc<String>,
}

impl From<&batch::Batch> for Batch {
    fn from(batch: &batch::Batch) -> Self {
        Self {
            tot: batch.tot,
            name: Arc::new(String::from(batch.cmd.tool.display_name())),
        }
    }
}

#[derive(Debug)]
pub(super) enum Event {
    Start { batch: Batch },
    Done { batch: Batch },
    Failed { output: Vec<u8> },
    Output { line: String },
}

#[derive(Debug)]
struct RunningBatches {
    min: usize,
    max: usize,
    tot: usize,
}

impl RunningBatches {
    fn new(tot: usize) -> Self {
        Self {
            min: 0,
            max: 0,
            tot,
        }
    }

    fn ok(&self) {
        debug_assert!(self.max < self.tot);
    }

    fn start(&mut self) {
        self.max += 1;
        self.ok();
    }

    fn done(&mut self) -> bool {
        self.min += 1;
        self.ok();
        self.min + 1 == self.tot
    }

    fn is_done(&self) -> bool {
        self.ok();
        self.min == self.tot || self.max == self.tot
    }

    fn num_running(&self) -> usize {
        self.ok();
        self.max.saturating_sub(self.min)
    }
}

pub(super) fn reporter<W: Write + ?Sized>(
    keep_going: bool,
    total_files: usize,
    start_time: Option<Instant>,
    rx: mpsc::Receiver<Event>,
    mut progress: Progress<'_, W>,
) {
    let mut displayed_batches = String::with_capacity(124);
    let mut running = BTreeMap::new();
    let mut errors = 0;
    loop {
        match rx.recv() {
            Ok(Event::Start { batch }) => {
                running
                    .entry(batch.name.clone())
                    .and_modify(|s: &mut RunningBatches| {
                        s.start();
                    })
                    .or_insert_with(|| RunningBatches::new(batch.tot));
                display_batches(&mut displayed_batches, progress.format, &running);
                debug_assert!(!displayed_batches.is_empty());
                progress.report(&displayed_batches);
            }
            Ok(Event::Output { line }) => {
                // Only display if there's exactly one command running
                let num_running = running
                    .values()
                    .map(RunningBatches::num_running)
                    .sum::<usize>();

                if num_running <= 1 {
                    display_batches(&mut displayed_batches, progress.format, &running);
                    if !displayed_batches.is_empty() && !line.is_empty() {
                        write!(&mut displayed_batches, ": {line}").ok();
                    }
                    progress.report(&displayed_batches);
                }
            }
            Ok(Event::Failed { output }) => {
                progress.fail(&output);
                errors += 1;
                if !keep_going {
                    final_report(total_files, errors, start_time, progress);
                    return;
                }
            }
            #[allow(clippy::unwrap_used)]
            Ok(Event::Done { batch }) => {
                let running_batches = running
                    .entry(batch.name.clone())
                    .or_insert_with(|| RunningBatches::new(batch.tot));
                let done = running_batches.done();
                if done {
                    running.remove(batch.name.as_ref());
                }
                progress.increment();
                if progress
                    .total
                    .as_ref()
                    .is_some_and(|t| progress.completed == *t)
                {
                    #[cfg(test)]
                    debug_assert!(running.values().all(RunningBatches::is_done));
                    debug_assert!(total_files >= 1);
                    final_report(total_files, errors, start_time, progress);
                    break;
                }
                if running
                    .values()
                    .map(RunningBatches::num_running)
                    .sum::<usize>()
                    != 0
                {
                    display_batches(&mut displayed_batches, progress.format, &running);
                    debug_assert!(!displayed_batches.is_empty());
                    progress.report(&displayed_batches);
                }
            }
            Err(e) => {
                error!("{e}");
                break;
            }
        }
    }
}

#[allow(clippy::unwrap_used)]
fn display_batches(
    s: &mut String,
    format: crate::progress::Format,
    running: &BTreeMap<Arc<String>, RunningBatches>,
) {
    s.clear();
    let term_width = match format {
        crate::progress::Format::Terminal(width) => width,
        _ => None,
    };
    let estimated_width = 20; // "[N/M] "
    let max_len = term_width.map(|w| w.saturating_sub(estimated_width).into());
    for (tool, rbs) in running {
        let min = rbs.min + 1;
        let max = rbs.max + 1;
        if rbs.is_done() || min > max {
            continue;
        }
        if !s.is_empty() {
            write!(s, ", ").unwrap();
        }
        write!(s, "{tool} (").unwrap();
        let all_batches = rbs.tot;
        if min == max {
            write!(s, "{min}/{all_batches})").unwrap();
        } else {
            write!(s, "{min}-{max}/{all_batches})").unwrap();
        }
        if max_len.is_some_and(|m| s.len() > m) {
            return;
        }
    }
}

fn final_report<W: Write + ?Sized>(
    total_files: usize,
    errors: usize,
    start_time: Option<Instant>,
    progress: Progress<'_, W>,
) {
    let duration_str = start_time.map(|t| format_duration(t.elapsed()));
    let has_errors = errors > 0;

    if total_files == 1 {
        if errors == 0 {
            progress.finalize(
                &format!("1 file linted{}", duration_str.as_deref().unwrap_or("")),
                has_errors,
            );
        } else if errors == 1 {
            progress.finalize(
                &format!("1 error{}", duration_str.as_deref().unwrap_or("")),
                has_errors,
            );
        } else {
            progress.finalize(&format!("{errors} errors"), has_errors);
        }
    } else if errors == 0 {
        progress.finalize(
            &format!(
                "{total_files} files linted{}",
                duration_str.as_deref().unwrap_or("")
            ),
            has_errors,
        );
    } else if errors == 1 {
        progress.finalize(
            &format!("1 error{}", duration_str.as_deref().unwrap_or("")),
            has_errors,
        );
    } else {
        progress.finalize(&format!("{errors} errors"), has_errors);
    }
}

fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs_f64();
    if secs < 0.05 {
        String::new()
    } else if secs < 10.0 {
        format!(" in {secs:.1}s")
    } else {
        format!(" in {secs:.0}s")
    }
}
