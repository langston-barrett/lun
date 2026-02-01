//! Format:
//!
//! ```text
//! [3/32] ttlint (1/8)
//! [4/32] ruff (5/10)
//! [5/32] clippy (1/1), ttlint (2/8), ttlint (3/8), ruff (6/10)...
//! ```

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt::Write as _,
    io::Write,
    ops,
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
    total: usize,
    running: BTreeSet<usize>,
}

impl RunningBatches {
    fn insert(&mut self, n: usize) -> bool {
        self.running.insert(n)
    }
}

impl ops::Deref for RunningBatches {
    type Target = BTreeSet<usize>;

    fn deref(&self) -> &Self::Target {
        &self.running
    }
}

impl ops::DerefMut for RunningBatches {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.running
    }
}

pub(super) fn reporter<W: Write + ?Sized>(
    keep_going: bool,
    n_threads: usize,
    rx: mpsc::Receiver<Event>,
    mut progress: Progress<'_, W>,
) {
    let mut displayed_batches = String::with_capacity(124);
    let mut running = BTreeMap::new();
    let mut seen = HashMap::with_capacity(n_threads);
    loop {
        match rx.recv() {
            Ok(Event::Start { batch }) => {
                let n = *seen
                    .entry(batch.name.clone())
                    .and_modify(|s| *s += 1)
                    .or_insert(0);
                running
                    .entry(batch.name.clone())
                    .and_modify(|s: &mut RunningBatches| {
                        let new = s.insert(n);
                        debug_assert!(new);
                    })
                    .or_insert_with(|| RunningBatches {
                        total: batch.tot,
                        running: BTreeSet::from([n]),
                    });
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
                let tot = running_batches.total;
                let min = *running_batches.iter().next().unwrap();
                running_batches.remove(&min);
                if min + 1 == tot {
                    running.remove(batch.name.as_ref());
                }
                progress.increment();
                if progress
                    .total
                    .as_ref()
                    .is_some_and(|t| progress.completed == *t)
                {
                    debug_assert!(running.values().all(|r| r.is_empty()));
                    progress.done();
                    break;
                }
                if running.values().map(|rbs| rbs.len()).sum::<usize>() != 0 {
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
        if rbs.is_empty() {
            continue;
        }
        if !s.is_empty() {
            write!(s, ", ").unwrap();
        }
        write!(s, "{tool} (").unwrap();
        let tot_batches = rbs.len();
        let all_batches = rbs.total;
        for (idx, b) in rbs.iter().enumerate() {
            write!(s, "{}", b + 1).unwrap();
            if idx + 1 != tot_batches {
                s.push(',');
            } else {
                write!(s, " / {all_batches})").unwrap();
            }
        }
        if s.len() > 60 {
            return;
        }
    }
}
