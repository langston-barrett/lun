use std::{num::NonZero, process};

use tracing::debug;

use crate::{config::Args, file, run::cmd};

#[derive(Debug)]
pub(super) struct Batch {
    pub(crate) tot: usize,
    pub(crate) cmd: cmd::Command,
}

impl Batch {
    fn one_of_one(cmd: cmd::Command) -> Self {
        Self { tot: 1, cmd }
    }

    pub(crate) fn to_command(&self) -> process::Command {
        self.cmd.to_command()
    }
}

pub(crate) fn display_cmd(c: &process::Command) -> String {
    if c.get_args().next().is_none() {
        c.get_program().display().to_string()
    } else {
        format!(
            "{} {}",
            c.get_program().display(),
            c.get_args()
                .map(|a| a.display().to_string())
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

pub(crate) fn create_jobs(
    commands: Vec<cmd::Command>,
    cores: NonZero<usize>,
    no_batch: bool,
) -> Vec<Batch> {
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

fn one_per_file(cmd: cmd::Command) -> Vec<Batch> {
    let tot = cmd.files.len();
    cmd.files
        .into_iter()
        .map(|file| Batch {
            tot,
            cmd: cmd::Command {
                tool: cmd.tool.clone(),
                files: vec![file],
            },
        })
        .collect()
}

fn unbatch(cmd: cmd::Command) -> Vec<Batch> {
    if cmd.files.is_empty() {
        return Vec::new();
    }
    // TODO: Is this Args::None case right?
    if cmd.tool.args == Args::None || cmd.tool.args == Args::All {
        return vec![Batch::one_of_one(cmd)];
    }
    one_per_file(cmd)
}

fn batch(mut cmd: cmd::Command, cores: NonZero<usize>) -> Vec<Batch> {
    debug_assert!(!cmd.files.is_empty());
    if cmd.files.is_empty() {
        return Vec::new();
    }
    let cores = cores.get();
    // TODO: Is this Args::None case right?
    if cmd.tool.args == Args::None || cmd.tool.args == Args::All {
        return vec![Batch::one_of_one(cmd)];
    }
    if cmd.tool.args == Args::One {
        return one_per_file(cmd);
    }
    if cores == 1 {
        return vec![Batch::one_of_one(cmd)];
    }
    debug_assert!(matches!(cmd.tool.args, Args::Many));
    if cmd.files.len() < cores {
        return one_per_file(cmd);
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

    let tot = jobs.len();
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
                let c = cmd.to_command();
                debug!("Batched {} (size: {sz})", display_cmd(&c));
                Some(Batch { tot, cmd })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{file, tool};
    use globset::GlobSetBuilder;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn make_test_tool(args: Args) -> Arc<tool::Tool> {
        let mut builder = GlobSetBuilder::new();
        builder.add(globset::Glob::new("*").unwrap());
        Arc::new(tool::Tool {
            name: Some("test".to_string()),
            cmd: "test --".to_string(),
            files: builder.build().unwrap(),
            ignore: None,
            args,
            include_unchanged: false,
            stamp: tool::Stamp(file::Xxhash(0)),
            cd: None,
            configs: Vec::new(),
            modifies_files: false,
        })
    }

    fn make_test_file(path: &str, size: usize) -> file::File {
        use xxhash_rust::xxh3::Xxh3;
        let mut hasher = Xxh3::new();
        hasher.update(path.as_bytes());
        file::File {
            path: PathBuf::from(path),
            size,
            metadata_stamp: file::Stamp(file::Xxhash(hasher.digest128())),
            mtime_stamp: file::Stamp(file::Xxhash(0)),
            content_stamp: Some(file::Stamp(file::Xxhash(0))),
        }
    }

    #[test]
    fn args_none_keeps_files_together() {
        let tool = make_test_tool(Args::None);
        let files = vec![
            make_test_file("test1.rs", 100),
            make_test_file("test2.rs", 100),
            make_test_file("test3.rs", 100),
        ];
        let cmd = cmd::Command { tool, files };

        let batches = unbatch(cmd);
        // Args::None should keep all files in one batch
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].cmd.files.len(), 3);
    }

    #[test]
    fn args_all_keeps_files_together() {
        let tool = make_test_tool(Args::All);
        let files = vec![
            make_test_file("test1.rs", 100),
            make_test_file("test2.rs", 100),
            make_test_file("test3.rs", 100),
        ];
        let cmd = cmd::Command { tool, files };

        let batches = unbatch(cmd);
        // Args::All should keep all files in one batch
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].cmd.files.len(), 3);
    }

    #[test]
    fn args_one_splits_files() {
        let tool = make_test_tool(Args::One);
        let files = vec![
            make_test_file("test1.rs", 100),
            make_test_file("test2.rs", 100),
            make_test_file("test3.rs", 100),
        ];
        let cmd = cmd::Command { tool, files };

        let batches = unbatch(cmd);
        // Args::One should split into one file per batch
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].cmd.files.len(), 1);
        assert_eq!(batches[1].cmd.files.len(), 1);
        assert_eq!(batches[2].cmd.files.len(), 1);
    }

    #[test]
    fn args_many_splits_files_in_unbatch() {
        let tool = make_test_tool(Args::Many);
        let files = vec![
            make_test_file("test1.rs", 100),
            make_test_file("test2.rs", 100),
            make_test_file("test3.rs", 100),
        ];
        let cmd = cmd::Command { tool, files };

        let batches = unbatch(cmd);
        // Args::Many in unbatch mode splits files
        assert_eq!(batches.len(), 3);
    }

    #[test]
    fn batch_respects_args_none() {
        let tool = make_test_tool(Args::None);
        let files = vec![
            make_test_file("test1.rs", 100),
            make_test_file("test2.rs", 100),
            make_test_file("test3.rs", 100),
        ];
        let cmd = cmd::Command { tool, files };

        let cores = NonZero::new(4).unwrap();
        let batches = batch(cmd, cores);
        // Args::None should keep all files together even with multiple cores
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].cmd.files.len(), 3);
    }

    #[test]
    fn batch_respects_args_all() {
        let tool = make_test_tool(Args::All);
        let files = vec![
            make_test_file("test1.rs", 100),
            make_test_file("test2.rs", 100),
            make_test_file("test3.rs", 100),
        ];
        let cmd = cmd::Command { tool, files };

        let cores = NonZero::new(4).unwrap();
        let batches = batch(cmd, cores);
        // Args::All should keep all files together even with multiple cores
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].cmd.files.len(), 3);
    }

    #[test]
    fn batch_splits_args_one() {
        let tool = make_test_tool(Args::One);
        let files = vec![
            make_test_file("test1.rs", 100),
            make_test_file("test2.rs", 100),
            make_test_file("test3.rs", 100),
        ];
        let cmd = cmd::Command { tool, files };

        let cores = NonZero::new(4).unwrap();
        let batches = batch(cmd, cores);
        assert_eq!(batches.len(), 3);
    }

    #[test]
    fn batch_splits_args_one_with_one_core() {
        let tool = make_test_tool(Args::One);
        let files = vec![
            make_test_file("test1.rs", 100),
            make_test_file("test2.rs", 100),
            make_test_file("test3.rs", 100),
        ];
        let cmd = cmd::Command { tool, files };

        let cores = NonZero::new(1).unwrap();
        let batches = batch(cmd, cores);
        // Args::One should split even with one core (unlike Args::Many)
        assert_eq!(batches.len(), 3);
    }
}
