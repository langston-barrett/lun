use std::{
    collections::HashSet,
    fs,
    io::{self, Write},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process,
    sync::mpsc,
    thread, time,
};

use anyhow::{Context, Result};
use notify::{Config as NotifyConfig, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{debug, trace, warn};

mod cmd;
mod collect;
mod exec;
mod filter;
mod job;
mod ninja;
mod plan;

#[cfg(test)]
mod test;

use crate::{
    Paths,
    cache::{self, CacheWriter},
    cli, config,
    file::{self, File},
    git, out,
    progress::{self, Format, Progress},
    tool,
    warn::{self, warns::Warns},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RunMode {
    Normal,
    Check,
    Fix,
}

impl From<&cli::Run> for RunMode {
    fn from(run: &cli::Run) -> Self {
        if run.fix {
            RunMode::Fix
        } else if run.check {
            RunMode::Check
        } else {
            RunMode::Normal
        }
    }
}

pub(crate) fn num_cores(cores: Option<NonZeroUsize>) -> NonZeroUsize {
    cores.unwrap_or_else(|| {
        thread::available_parallelism().unwrap_or(const { NonZeroUsize::new(1).unwrap() })
    })
}

fn include_tool(tool: &config::Tool, run: &cli::Run) -> bool {
    let skip = tool
        .name
        .as_ref()
        .is_none_or(|n| !run.skip_tool.contains(n));
    let only = run.only_tool.is_empty()
        || tool
            .name
            .as_ref()
            .is_some_and(|n| run.only_tool.contains(n));
    skip && only
}

fn filter_tools(
    run: &cli::Run,
    config: &config::Config,
    mode: RunMode,
    out_config: out::Config,
) -> Result<Vec<tool::Tool>> {
    let careful = run.careful || config.careful;
    let effective_ignore = config.effective_ignore();
    let mut tools = Vec::new();

    if !run.format {
        for linter in &config.linter {
            if include_tool(&linter.tool, run) {
                tools.push(linter.clone().into_tool(
                    mode,
                    careful,
                    out_config,
                    &effective_ignore,
                )?);
            }
        }
    }

    for formatter in &config.formatter {
        if include_tool(&formatter.tool, run) {
            tools.push(formatter.clone().into_tool(
                mode,
                careful,
                out_config,
                &effective_ignore,
            )?);
        }
    }

    Ok(tools)
}

#[derive(Debug, Clone)]
struct Config {
    refs: Vec<git::Ref>,
    cache: PathBuf,
    config_path: PathBuf,
    cwd: PathBuf,
    cores: NonZeroUsize,
    dry_run: bool,
    mtime: bool,
    ninja: bool,
    no_batch: bool,
    no_capture: bool,
    no_cache: bool,
    tools: Vec<tool::Tool>,
    progress_format: Format,
    keep_going: bool,
    then: Option<String>,
    r#else: Option<String>,
    cache_size: Option<usize>,
}

impl Config {
    fn collect_files(
        &self,
        staged: bool,
        vcs: bool,
        only: &[String],
        skip: &[String],
        out: &mut (impl Write + ?Sized),
    ) -> Result<Vec<File>> {
        collect::go(
            &self.cwd,
            &self.cache,
            self.progress_format,
            out,
            only,
            skip,
            staged,
            vcs,
        )
    }
}

fn mk_config(
    paths: &Paths,
    out_config: out::Config,
    run: &cli::Run,
    config: &config::Config,
) -> Result<Config> {
    let mode = RunMode::from(run);
    let refs = if run.no_refs || run.fresh {
        Vec::new()
    } else if !run.refs.is_empty() {
        run.refs.clone()
    } else {
        config.refs.clone()
    };
    let mtime = config.mtime && !run.no_mtime;
    let tools = filter_tools(run, config, mode, out_config)?;
    Ok(Config {
        refs,
        cache: paths.cache.clone(),
        config_path: paths.config.clone().unwrap_or_default(),
        cwd: paths.cwd.clone(),
        cores: num_cores(run.jobs.or(config.cores)),
        dry_run: run.dry_run,
        mtime,
        ninja: run.ninja || config.ninja.unwrap_or(false),
        no_batch: run.no_batch,
        no_capture: run.no_capture,
        no_cache: run.no_cache || run.fresh,
        tools,
        progress_format: Format::new(out_config),
        keep_going: run.keep_going,
        then: run.then.clone(),
        r#else: run.r#else.clone(),
        cache_size: run.cache_size.or(config.cache_size),
    })
}

#[derive(Debug, PartialEq)]
pub(crate) enum RunResult {
    AllGood { cmds: usize, files: usize },
    Errors,
}

impl From<RunResult> for bool {
    fn from(value: RunResult) -> Self {
        Self::from(&value)
    }
}

impl From<&RunResult> for bool {
    fn from(value: &RunResult) -> Self {
        match value {
            RunResult::AllGood { .. } => true,
            RunResult::Errors => false,
        }
    }
}

fn run(
    config: &Config,
    files: &[File],
    lints: &Warns,
    out: &mut (impl Write + Send),
) -> Result<RunResult> {
    trace!(?config);
    debug_assert!(files.iter().all(|f| f.content_stamp.is_none()));
    let cache_file = config.cache.join("cache");
    let mut cache = if config.no_cache {
        cache::HashCache::new(PathBuf::from("/dev/null"), 0)
    } else {
        cache::HashCache::from_file(&cache_file, config.cache_size)?
    };
    let plan_total = files.len() * config.tools.len();
    let mut plan_progress = Progress::new(config.progress_format, Some(plan_total), out);
    let jobs = plan::plan(
        &mut cache,
        &config.tools,
        files,
        &config.refs,
        Some(config.config_path.as_path()),
        &config.cwd,
        config.cores,
        config.no_batch,
        config.mtime,
        &mut plan_progress,
    );
    plan_progress.done();
    if !config.no_cache {
        cache.flush()?;
    }
    let no_jobs = jobs.is_empty();
    let n_jobs = jobs.len();
    let files_linted = jobs
        .iter()
        .flat_map(|job| job.files.iter().map(|f| &f.path))
        .collect::<HashSet<_>>()
        .len();
    let result = do_exec(config, &mut cache, jobs, out);
    if !no_jobs && !config.no_cache {
        let cache_full = cache.flush()?;
        warn::check_cache_usage(lints, cache.entries_added, cache.max_entries)?;
        warn::check_cache_full(lints, cache_full)?;
    }
    let result = match result {
        _ if config.dry_run => Ok(RunResult::AllGood { cmds: 0, files: 0 }),
        Ok(true) => Ok(RunResult::AllGood {
            cmds: n_jobs,
            files: files_linted,
        }),
        Ok(false) => Ok(RunResult::Errors),
        Err(e) => {
            // Write the final newline that report_result would otherwise handle
            if matches!(config.progress_format, Format::Terminal) {
                drop(out.write(b"\n"));
            }
            Err(e)
        }
    }?;
    report_result(config.progress_format, &result, out);
    then_else(config, &result)?;
    Ok(result)
}

fn do_exec(
    config: &Config,
    cache: &mut (impl CacheWriter + ?Sized),
    jobs: Vec<cmd::Command>,
    out: &mut (impl Write + Send),
) -> Result<bool> {
    if config.ninja {
        ninja::exec(
            cache,
            config.cache.as_path(),
            jobs,
            config.cores,
            config.dry_run,
            config.no_capture,
            config.keep_going,
            config.mtime,
        )
    } else if config.dry_run {
        Ok(true)
    } else {
        exec::exec(
            cache,
            jobs,
            config.cores,
            config.no_capture,
            config.progress_format,
            config.keep_going,
            config.mtime,
            out,
        )
    }
}

fn then_else(config: &Config, result: &RunResult) -> Result<(), anyhow::Error> {
    let success = bool::from(result);
    let (which, cmd_to_run) = if success {
        ("then", config.then.as_deref())
    } else {
        ("else", config.r#else.as_deref())
    };
    if let Some(cmd) = cmd_to_run {
        let mut bash_cmd = process::Command::new("bash");
        bash_cmd.arg("-c").arg(cmd);
        let status = bash_cmd
            .status()
            .with_context(|| format!("Failed to execute `{which}` command: {cmd}"))?;
        if !status.success() {
            return Ok(());
        }
    }
    Ok(())
}

pub(crate) fn go(
    paths: &Paths,
    run_cli: &cli::Run,
    config: &config::Config,
    lints: &Warns,
    out_config: out::Config,
    out: &mut (impl Write + Send),
) -> std::result::Result<RunResult, anyhow::Error> {
    lint(run_cli, config, lints)?;
    fs::create_dir_all(&paths.cache)?; // just to create the dir
    if run_cli.watch {
        watch(paths, out_config, run_cli, config, lints, out)?;
        Ok(RunResult::AllGood { cmds: 0, files: 0 })
    } else {
        let config = mk_config(paths, out_config, run_cli, config)?;
        let files = config.collect_files(
            run_cli.staged,
            run_cli.vcs,
            &run_cli.only_files,
            &run_cli.skip_files,
            out,
        )?;
        let result = run(&config, &files, lints, out);
        #[cfg(debug_assertions)]
        {
            let debug_cache = paths.cache.join("debug");
            drop(fs::remove_dir_all(&debug_cache));
            drop(fs::create_dir_all(&debug_cache));
            let mut debug_config = config.clone();
            debug_config.cache = debug_cache;
            let debug_result = run(&debug_config, &files, lints, &mut io::sink());
            debug_assert!(
                match (result.as_ref(), debug_result.as_ref()) {
                    (Ok(r1), Ok(r2)) => bool::from(r1) == bool::from(r2),
                    _ => true,
                },
                "Results differ between normal and debug cache"
            );
        }
        result
    }
}

fn lint(run_cli: &cli::Run, config: &config::Config, lints: &Warns) -> Result<(), anyhow::Error> {
    warn::check_unknown_tools(lints, &run_cli.skip_tool, &run_cli.only_tool, config)?;
    warn::check_unlisted_config(lints, config)?;
    warn::check_no_files(lints, config)?;
    warn::check_careful(lints, run_cli.careful, config.careful)?;
    warn::check_mtime(lints, run_cli.no_mtime, config.mtime)?;
    warn::check_refs(lints, &run_cli.refs, &config.refs)?;
    Ok(())
}

fn clear_term() {
    print!("\x1B[2J\x1B[1;1H");
    drop(io::stdout().flush());
}

// TODO: A "true" watch mode that updates an internal model of the filesystem
// using the events from `notify`. See e.g.,
// https://github.com/astral-sh/ruff/blob/main/crates/ty_project/src/watch/watcher.rs
fn watch(
    paths: &Paths,
    out_config: out::Config,
    run_cli: &cli::Run,
    config: &config::Config,
    lints: &Warns,
    out: &mut (impl Write + Send),
) -> Result<bool> {
    let config = mk_config(paths, out_config, run_cli, config)?;
    let files = config.collect_files(
        run_cli.staged,
        run_cli.vcs,
        &run_cli.only_files,
        &run_cli.skip_files,
        out,
    )?;
    run(&config, &files, lints, out)?;

    let initial_config_hash = paths
        .config
        .as_ref()
        .and_then(|p| fs::read(p).ok())
        .map(|contents| file::compute_hash(&contents));

    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |res| {
            if let Err(e) = tx.send(res) {
                warn!("Error sending watch event: {e}");
            }
        },
        NotifyConfig::default().with_poll_interval(time::Duration::from_secs(5)),
    )
    .context("Failed to create file watcher")?;

    watcher
        .watch(&paths.cwd, RecursiveMode::Recursive)
        .context("Failed to start watching directory")?;

    debug!("Watching for file changes...");
    let mut last_run = time::Instant::now();
    loop {
        let mut needed = false;
        let ev = rx.recv().context("File watcher channel error")?;
        needed |= process_event(ev)?;
        while let Ok(ev) = rx.try_recv() {
            needed |= process_event(ev)?;
        }
        if needed && last_run.elapsed() > time::Duration::from_millis(50) {
            clear_term();
            warn_if_config_changed(paths.config.as_deref(), initial_config_hash);
            thread::sleep(time::Duration::from_millis(20));
            let files = config.collect_files(
                run_cli.staged,
                run_cli.vcs,
                &run_cli.only_files,
                &run_cli.skip_files,
                out,
            )?;
            run(&config, &files, lints, out)?;
        }
        last_run = time::Instant::now();
    }
}

fn report_result(progress_format: Format, res: &RunResult, out: &mut (impl Write + ?Sized)) {
    let prefix = progress::prefix(progress_format);
    match res {
        RunResult::AllGood { cmds, files: 0 } => {
            debug_assert_eq!(*cmds, 0);
            drop(writeln!(out, "{prefix}[{cmds}/{cmds}] 0 files linted"));
        }
        RunResult::AllGood { cmds, files: 1 } => {
            drop(writeln!(out, "{prefix}[{cmds}/{cmds}] 1 file linted"));
        }
        RunResult::AllGood { cmds, files } => {
            drop(writeln!(
                out,
                "{prefix}[{cmds}/{cmds}] {files} files linted"
            ));
        }
        RunResult::Errors => (), // output is mirrored to std{out,err}
    }
}

fn process_event(ev: Result<notify::Event, notify::Error>) -> Result<bool> {
    let ev = ev.context("File watcher error")?;
    trace!("Filesystem event: {:?} {:?}", ev.kind, ev.paths);
    Ok(need_rerun(&ev))
}

fn need_rerun(event: &notify::Event) -> bool {
    if matches!(event.kind, EventKind::Access(_)) {
        return false;
    }
    let ignored_prefixes = [".lun", ".git", "target"];
    let all_paths_ignored = event.paths.iter().all(|path| {
        ignored_prefixes.iter().any(|prefix| {
            path.components()
                .any(|component| component.as_os_str() == *prefix)
        })
    });
    !all_paths_ignored
}

fn warn_if_config_changed(config: Option<&Path>, initial_config_hash: Option<file::Xxhash>) {
    if let Some(config_path) = config
        && let Some(initial_hash) = initial_config_hash
        && let Ok(content) = fs::read(config_path)
    {
        let hash = file::compute_hash(&content);
        if hash != initial_hash {
            warn!("Config file changed! Please restart `lun`.");
        }
    }
}
