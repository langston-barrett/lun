use std::num::NonZero;

use tracing::debug;

use crate::{cmd, config::Granularity, file, run::RunMode};

pub(crate) fn create_jobs(
    commands: Vec<cmd::Command>,
    cores: NonZero<usize>,
    no_batch: bool,
) -> Vec<cmd::Command> {
    if commands.is_empty() {
        debug!("No commands to execute");
        return Vec::new();
    }
    let mut batches = Vec::with_capacity(commands.len() /* min */);
    for cmd in commands {
        if no_batch {
            batches.extend(unbatch(cmd));
        } else {
            batches.extend(batch(cmd, cores));
        }
    }
    batches
}

fn unbatch(cmd: cmd::Command) -> Vec<cmd::Command> {
    if cmd.files.is_empty() {
        return Vec::new();
    }
    if cmd.files.len() == 1 || cmd.tool.granularity == Granularity::Batch {
        return vec![cmd];
    }
    cmd.files
        .into_iter()
        .map(|file| cmd::Command {
            tool: cmd.tool.clone(),
            files: vec![file],
        })
        .collect()
}

fn batch(mut cmd: cmd::Command, cores: NonZero<usize>) -> Vec<cmd::Command> {
    debug_assert!(!cmd.files.is_empty());
    if cmd.files.is_empty() {
        return Vec::new();
    }
    let cores = cores.get();
    if cmd.files.len() == 1 || cmd.tool.granularity == Granularity::Batch || cores == 1 {
        return vec![cmd];
    }
    if cmd.files.len() < cores {
        return cmd
            .files
            .into_iter()
            .map(|file| cmd::Command {
                tool: cmd.tool.clone(),
                files: vec![file],
            })
            .collect();
    }

    cmd.files.sort_by(|a, b| b.size.cmp(&a.size));
    let mut jobs: Vec<(Vec<file::File>, usize)> = (0..cores).map(|_| (Vec::new(), 0)).collect();
    // Distribute files to jobs using a greedy algorithm
    for file in cmd.files {
        // Find the batch with the smallest total size
        let smallest_batch_idx = jobs
            .iter()
            .enumerate()
            .min_by_key(|(_, (_, total_size))| *total_size)
            .map_or(0, |(idx, _)| idx);

        jobs[smallest_batch_idx].1 += file.size;
        jobs[smallest_batch_idx].0.push(file);
    }

    jobs.into_iter()
        .filter_map(|(mut files, sz)| {
            if files.is_empty() {
                None
            } else {
                files.sort_by(|a, b| a.path.cmp(&b.path));
                let cmd = cmd::Command {
                    tool: cmd.tool.clone(),
                    files,
                };
                let c = cmd.to_command(RunMode::Normal);
                debug!(
                    "Batched {} {} (size: {sz})",
                    c.get_program().display(),
                    c.get_args()
                        .map(|a| a.display().to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                Some(cmd)
            }
        })
        .collect()
}
