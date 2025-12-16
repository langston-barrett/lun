use std::{
    collections::HashSet,
    fs,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process,
    sync::mpsc,
    thread, time,
};

use anyhow::{Context, Result};
use globset::Glob;
use notify::{Config as NotifyConfig, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{debug, trace, warn};

use crate::{
    cache::{self, CacheWriter},
    cli, config, exec, file, ninja, plan, staged, tool,
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

fn collect_files(
    cli: &cli::Cli,
    run: &cli::Run,
    progress_format: exec::ProgressFormat,
) -> Result<Vec<file::File>, anyhow::Error> {
    let mut files = if run.staged {
        staged::collect_staged_files()?
    } else {
        file::collect_files(Path::new("."), &cli.cache, progress_format)?
    };
    filter_files(&mut files, &run.only_files, &run.skip_files)?;
    Ok(files)
}

fn only_matchers(only_patterns: &[String]) -> Result<Vec<globset::GlobMatcher>, anyhow::Error> {
    let only = only_patterns
        .iter()
        .map(|pattern| {
            Glob::new(pattern)
                .with_context(|| format!("Invalid `only` glob pattern: {pattern}"))
                .map(|g| g.compile_matcher())
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(only)
}

fn skip_matchers(skip_patterns: &[String]) -> Result<Vec<globset::GlobMatcher>, anyhow::Error> {
    let skip = skip_patterns
        .iter()
        .map(|pattern| {
            Glob::new(pattern)
                .with_context(|| format!("Invalid `skip` glob pattern: {pattern}"))
                .map(|g| g.compile_matcher())
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(skip)
}

pub(crate) fn filter_files(
    files: &mut Vec<file::File>,
    only_patterns: &[String],
    skip_patterns: &[String],
) -> Result<()> {
    if only_patterns.is_empty() && skip_patterns.is_empty() {
        return Ok(());
    }

    let only = only_matchers(only_patterns)?;
    let skip = skip_matchers(skip_patterns)?;

    files.retain(|file| {
        let path = file.path.as_path();
        if !only.is_empty() && !only.iter().any(|m| m.is_match(path)) {
            return false;
        }
        if skip.iter().any(|m| m.is_match(path)) {
            return false;
        }
        true
    });

    Ok(())
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
    color: cli::log::Color,
) -> Result<Vec<tool::Tool>> {
    let careful = run.careful || config.careful;
    let mut tools = Vec::new();

    if !run.format {
        for linter in &config.linter {
            if include_tool(&linter.tool, run) {
                tools.push(
                    linter
                        .clone()
                        .into_tool(mode, careful, color, &config.ignore)?,
                );
            }
        }
    }

    for formatter in &config.formatter {
        if include_tool(&formatter.tool, run) {
            tools.push(
                formatter
                    .clone()
                    .into_tool(mode, careful, color, &config.ignore)?,
            );
        }
    }

    Ok(tools)
}

#[derive(Debug, Clone)]
struct Config {
    refs: Vec<String>,
    cache: PathBuf,
    cores: NonZeroUsize,
    dry_run: bool,
    files: Vec<file::File>,
    mtime: bool,
    ninja: bool,
    no_batch: bool,
    no_capture: bool,
    no_cache: bool,
    tools: Vec<tool::Tool>,
    show_progress: exec::ProgressFormat,
    keep_going: bool,
    then: Option<String>,
    r#else: Option<String>,
    cache_size: Option<usize>,
}

fn mk_config(cli: &cli::Cli, run: &cli::Run, config: &config::Config) -> Result<Config> {
    let mode = RunMode::from(run);
    let show_progress = if cli.log.quiet == cli.log.verbose {
        // verbosity == info
        exec::ProgressFormat::Yes
    } else if cli.log.quiet <= cli.log.verbose {
        exec::ProgressFormat::Newline
    } else {
        exec::ProgressFormat::No
    };
    let refs = if run.no_refs || run.fresh {
        Vec::new()
    } else if !run.refs.is_empty() {
        run.refs.clone()
    } else {
        config.refs.clone()
    };
    let mtime = config.mtime && !run.no_mtime;
    Ok(Config {
        refs,
        cache: cli.cache.clone(),
        cores: num_cores(run.jobs.or(config.cores)),
        dry_run: run.dry_run,
        files: collect_files(cli, run, show_progress)?,
        mtime,
        ninja: run.ninja || config.ninja.unwrap_or(false),
        no_batch: run.no_batch,
        no_capture: run.no_capture,
        no_cache: run.no_cache || run.fresh,
        tools: filter_tools(run, config, mode, cli.log.color)?,
        show_progress,
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

fn run(config: &Config) -> Result<RunResult> {
    trace!(?config);
    debug_assert!(config.files.iter().all(|f| f.content_stamp.is_none()));
    let cache_file = config.cache.join("cache");
    let mut cache: &mut dyn cache::Cache = if config.no_cache {
        &mut cache::NopCache
    } else {
        &mut cache::HashCache::from_file(&cache_file, config.cache_size)?
    };
    let jobs = plan::plan(
        cache,
        &config.tools,
        &config.files,
        &config.refs,
        config.cores,
        config.no_batch,
        config.mtime,
    )?;
    cache.flush()?;
    let no_jobs = jobs.is_empty();
    let n_jobs = jobs.len();
    let files_linted = jobs
        .iter()
        .flat_map(|job| job.files.iter().map(|f| &f.path))
        .collect::<HashSet<_>>()
        .len();
    let result = do_exec(config, &mut cache, jobs);
    if !no_jobs {
        cache.flush()?;
    }
    let result = match result {
        _ if config.dry_run => Ok(RunResult::AllGood { cmds: 0, files: 0 }),
        Ok(true) => Ok(RunResult::AllGood {
            cmds: n_jobs,
            files: files_linted,
        }),
        Ok(false) => Ok(RunResult::Errors),
        Err(e) => Err(e),
    }?;
    report_result(&result);
    then_else(config, &result)?;
    Ok(result)
}

fn do_exec(
    config: &Config,
    cache: &mut impl CacheWriter,
    jobs: Vec<crate::cmd::Command>,
) -> std::result::Result<bool, anyhow::Error> {
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
            config.show_progress,
            config.keep_going,
            config.mtime,
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
    };
    Ok(())
}

pub(crate) fn go(
    cli: &cli::Cli,
    run_cli: &cli::Run,
    config: &config::Config,
    lints: &Warns,
) -> std::result::Result<RunResult, anyhow::Error> {
    lint(run_cli, config, lints)?;
    fs::create_dir_all(&cli.cache)?; // just to create the dir
    if run_cli.watch {
        watch(cli, run_cli, config)?;
        Ok(RunResult::AllGood { cmds: 0, files: 0 })
    } else {
        let config = mk_config(cli, run_cli, config)?;
        let result = run(&config);
        #[cfg(debug_assertions)]
        {
            let debug_cache = cli.cache.join("debug");
            drop(fs::remove_dir_all(&debug_cache));
            drop(fs::create_dir_all(&debug_cache));
            let mut debug_config = config.clone();
            debug_config.cache = debug_cache;
            let debug_result = run(&debug_config);
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
}

// TODO: A "true" watch mode that updates an internal model of the filesystem
// using the events from `notify`. See e.g.,
// https://github.com/astral-sh/ruff/blob/main/crates/ty_project/src/watch/watcher.rs
fn watch(cli: &cli::Cli, run_cli: &cli::Run, config: &config::Config) -> Result<bool> {
    let mut config = mk_config(cli, run_cli, config)?;
    run(&config)?;

    let initial_config_hash = fs::read(&cli.config)
        .ok()
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

    let cwd = Path::new(".");
    watcher
        .watch(cwd, RecursiveMode::Recursive)
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
            warn_if_config_changed(&cli.config, initial_config_hash);
            thread::sleep(time::Duration::from_millis(20));
            config.files = collect_files(cli, run_cli, config.show_progress)?;
            run(&config)?;
        }
        last_run = time::Instant::now();
    }
}

fn report_result(res: &RunResult) {
    match res {
        RunResult::AllGood { cmds, files: 0 } => {
            debug_assert_eq!(*cmds, 0);
            eprintln!("\x1b[2K\r[{cmds}/{cmds}] 0 files linted");
        }
        RunResult::AllGood { cmds, files: 1 } => {
            eprintln!("\x1b[2K\r[{cmds}/{cmds}] 1 file linted");
        }
        RunResult::AllGood { cmds, files } => {
            eprintln!("\x1b[2K\r[{cmds}/{cmds}] {files} files linted");
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

fn warn_if_config_changed(config: &Path, initial_config_hash: Option<file::Xxhash>) {
    if let Some(initial_hash) = initial_config_hash
        && let Ok(content) = fs::read(config)
    {
        let hash = file::compute_hash(&content);
        if hash != initial_hash {
            warn!("Config file changed! Please restart `lun`.");
        }
    }
}
