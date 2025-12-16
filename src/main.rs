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

use anyhow::Result;
use clap::Parser as _;
use std::process;
use tracing::trace;

#[cfg(feature = "dhat")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

pub(crate) fn go(cli: cli::Cli, config: Option<config::Config>) -> Result<bool> {
    let lints = warn::warns::Warns::from_cli_and_config(&cli.warn, config.as_ref())?;
    match &cli.command {
        cli::Command::Cache(cache_cmd) => match &cache_cmd.command {
            cli::CacheCommand::Rm => {
                cache::rm(&cli.cache)?;
                Ok(true)
            }
            cli::CacheCommand::Gc { size } => {
                let cache_file = cli.cache.join("cache");
                cache::gc(&cache_file, *size)?;
                Ok(true)
            }
        },
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
