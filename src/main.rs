#![cfg_attr(not(test), warn(clippy::expect_used))]
#![cfg_attr(not(test), warn(clippy::panic))]
#![cfg_attr(not(test), warn(clippy::unwrap_used))]

mod add;
mod cache;
mod cli;
mod cmd;
mod config;
mod entry;
mod exec;
mod file;
mod git;
mod init;
mod job;
mod known;
mod log;
mod ninja;
mod plan;
mod progress;
mod run;
mod staged;
mod tool;
mod warn;

#[cfg(test)]
mod test;

use anyhow::Result;
use clap::Parser as _;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use tracing::{debug, trace};

#[cfg(feature = "dhat")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// Resolved paths for config, cache, and working directory.
#[derive(Debug)]
pub(crate) struct Paths {
    pub(crate) config: Option<PathBuf>,
    pub(crate) cache: PathBuf,
    pub(crate) cwd: PathBuf,
}

impl Paths {
    /// Resolve config, cache, and cwd paths from CLI options.
    ///
    /// - If `--config` is specified, use that path
    /// - Otherwise, search parent directories for `lun.toml`
    /// - If `--cache` is specified, use that path
    /// - Otherwise, use `.lun` relative to the config file's directory (or cwd if no config)
    /// - `cwd` is set to the current working directory
    pub(crate) fn resolve(cli: &cli::Cli) -> Self {
        let config = cli.config.clone().or_else(config::Config::find);

        let cache = cli.cache.clone().unwrap_or_else(|| {
            config
                .as_ref()
                .and_then(|p| p.parent())
                .map_or_else(|| PathBuf::from(".lun"), |p| p.join(".lun"))
        });

        let cwd = PathBuf::from(".");

        Self { config, cache, cwd }
    }
}

pub(crate) fn go(
    cli: cli::Cli,
    paths: &Paths,
    config: Option<config::Config>,
    out: &mut (impl Write + Send),
) -> Result<bool> {
    let lints = warn::warns::Warns::from_cli_and_config(&cli.warn, config.as_ref())?;
    match &cli.command {
        cli::Command::Cache(cache_cmd) => match &cache_cmd.command {
            cli::CacheCommand::Rm => {
                cache::rm(&paths.cache)?;
                Ok(true)
            }
            cli::CacheCommand::Gc { size } => {
                let cache_file = paths.cache.join("cache");
                cache::gc(&cache_file, *size)?;
                Ok(true)
            }
            cli::CacheCommand::Stats => {
                let cache_file = paths.cache.join("cache");
                cache::stats(&cache_file)?;
                Ok(true)
            }
            cli::CacheCommand::Entry(entry_cmd) => {
                let cache_file = paths.cache.join("cache");
                match &entry_cmd.command {
                    cli::CacheEntryCommand::Add { key, files } => {
                        entry::add(&cache_file, key, files)?;
                        Ok(true)
                    }
                    cli::CacheEntryCommand::Get {
                        key,
                        files,
                        null_separated,
                    } => {
                        entry::get(&cache_file, key, files, *null_separated)?;
                        Ok(true)
                    }
                    cli::CacheEntryCommand::Rm { key, files } => {
                        entry::rm(&cache_file, key, files)?;
                        Ok(true)
                    }
                }
            }
        },
        cli::Command::Run(run) => {
            let config = config
                .ok_or_else(|| anyhow::anyhow!("Config file not found. Hint: try `lun init`."))?;
            run::go(&cli, paths, run, &config, &lints, out).map(bool::from)
        }
        cli::Command::Init(init) => {
            let config_path = paths
                .config
                .as_deref()
                .unwrap_or_else(|| Path::new(config::CONFIG_FILE_NAME));
            init::go(config_path, init)?;
            Ok(true)
        }
        cli::Command::Add(add) => {
            let config_path = paths
                .config
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Config file not found. Hint: try `lun init`."))?;
            add::go(config_path, add)?;
            Ok(true)
        }
        cli::Command::Warns { warn } => {
            warn::warns(warn.as_deref())?;
            Ok(true)
        }
        cli::Command::Known => {
            #[derive(serde::Serialize)]
            struct KnownTools {
                linter: Vec<config::Linter>,
                formatter: Vec<config::Formatter>,
            }
            let tools = KnownTools {
                linter: known::known_linters(),
                formatter: known::known_formatters(),
            };
            let toml = toml::to_string_pretty(&tools)?;
            write!(out, "{toml}")?;
            Ok(true)
        }
    }
}

fn main() -> Result<()> {
    #[cfg(feature = "dhat")]
    let _profiler = dhat::Profiler::new_heap();

    let cli = cli::Cli::parse();
    log::init_tracing(cli.log);
    debug!("version = {}", env!("CARGO_PKG_VERSION"));
    trace!(?cli);
    let paths = Paths::resolve(&cli);
    trace!(?paths);
    let config = match &paths.config {
        Some(path) => config::Config::load(path)?,
        None => None,
    };
    trace!(?config);
    let ok = go(cli, &paths, config, &mut io::stderr())?;
    if !ok {
        process::exit(1);
    }
    Ok(())
}
