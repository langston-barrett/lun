#![cfg_attr(not(test), warn(clippy::expect_used))]
#![cfg_attr(not(test), warn(clippy::panic))]
#![cfg_attr(not(test), warn(clippy::unwrap_used))]

mod add;
mod cache;
mod cli;
mod cmd;
mod config;
mod exec;
mod file;
mod git;
mod init;
mod job;
mod known;
mod log;
mod ninja;
mod plan;
mod run;
mod staged;
mod tool;
mod warn;

#[cfg(test)]
mod test;

use anyhow::{Context, Result};
use clap::Parser as _;
use std::{fs, path::Path, process};
use tracing::{debug, trace};

#[cfg(feature = "dhat")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn clean(path: &Path) -> Result<(), anyhow::Error> {
    if path.exists() {
        fs::remove_dir_all(path)
            .with_context(|| format!("Failed to remove cache: {}", path.display()))?;
        debug!("Cache removed from {}", path.display());
    }
    Ok(())
}

pub(crate) fn go(cli: cli::Cli, config: Option<config::Config>) -> Result<bool> {
    let lints = warn::warns::Warns::from_cli_and_config(&cli.warn, config.as_ref())?;
    match &cli.command {
        cli::Command::Clean => {
            clean(&cli.cache)?;
            Ok(true)
        }
        cli::Command::Run(run) => {
            let config = config.ok_or_else(|| anyhow::anyhow!("Config file not found"))?;
            run::go(&cli, run, &config, &lints).map(bool::from)
        }
        cli::Command::Init(init) => {
            init::go(&cli.config, init)?;
            Ok(true)
        }
        cli::Command::Add(add) => {
            add::go(&cli.config, add)?;
            Ok(true)
        }
        cli::Command::Warns { warn } => {
            warn::warns(warn.as_deref())?;
            Ok(true)
        }
    }
}

fn main() -> Result<()> {
    #[cfg(feature = "dhat")]
    let _profiler = dhat::Profiler::new_heap();

    let cli = cli::Cli::parse();
    log::init_tracing(cli.log);
    trace!(?cli);
    let config = config::Config::load(&cli.config)?;
    trace!(?config);
    let ok = go(cli, config)?;
    if !ok {
        process::exit(1);
    }
    Ok(())
}
