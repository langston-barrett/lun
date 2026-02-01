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
};

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
    rx: mpsc::Receiver<Event>,
    mut progress: Progress<'_, W>,
) {
    let mut displayed_batches = String::with_capacity(124);
    let mut running = BTreeMap::new();
    loop {
        match rx.recv() {
            Ok(Event::Start { batch }) => {
                running
                    .entry(batch.name.clone())
                    .and_modify(|s: &mut RunningBatches| {
                        s.start();
                    })
                    .or_insert_with(|| RunningBatches::new(batch.tot));
                display_batches(&mut displayed_batches, &running);
                debug_assert!(!displayed_batches.is_empty());
                progress.report(&displayed_batches);
            }
            Ok(Event::Failed { output }) => {
                let mut msg = b"Command failed:".to_vec();
                msg.extend_from_slice(&output);
                progress.fail(&msg);
                if !keep_going {
                    return;
                }
            }
            #[allow(clippy::unwrap_used)]
            Ok(Event::Done { batch }) => {
                let running_batches = running.get_mut(batch.name.as_ref()).unwrap();
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
                    debug_assert!(running.values().all(RunningBatches::is_done));
                    progress.done();
                    break;
                }
                if running
                    .values()
                    .map(RunningBatches::num_running)
                    .sum::<usize>()
                    != 0
                {
                    display_batches(&mut displayed_batches, &running);
                    debug_assert!(!displayed_batches.is_empty());
                    progress.report(&displayed_batches);
                }
            }
            Err(_) => {
                // Mark done to prevent drop from adding a newline;
                // final newline printing happens in `run::report_result`
                progress.done();
                break;
            }
        }
    }
}

#[allow(clippy::unwrap_used)]
fn display_batches(s: &mut String, running: &BTreeMap<Arc<String>, RunningBatches>) {
    s.clear();
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
        if s.len() > 60 {
            return;
        }
    }
}
