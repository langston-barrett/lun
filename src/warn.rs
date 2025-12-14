use std::collections::HashSet;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::bail;
use tracing::{error, warn};

use crate::config;
use crate::known;

pub(crate) mod group;
pub(crate) mod level;
#[allow(clippy::module_inception)]
pub(crate) mod warn;
pub(crate) mod warns;

use group::Group;
use warn::Warn;
use warns::Warns;

pub(crate) fn warns(name: Option<&str>) -> anyhow::Result<()> {
    if let Some(name) = name {
        let warn = Warn::from_str(name).map_err(|_| anyhow::anyhow!("Unknown lint: {name}"))?;
        print!("{}", warn.doc());
    } else {
        for warn in Warn::all() {
            println!(
                "{}: {} ({})",
                warn.as_str(),
                warn.help(),
                warn.default_level(),
            );
        }
        for group in Group::all() {
            println!();
            println!("{}:", group.into_str());
            for lint in group.warns() {
                println!("{}", lint.as_str());
            }
        }
    }
    Ok(())
}

pub(crate) fn check_unknown_tools(
    lints: &Warns,
    skip_tool: &[String],
    only_tool: &[String],
    config: &config::Config,
) -> anyhow::Result<()> {
    let level = lints.level(Warn::UnknownTool);
    if matches!(level, level::Level::Allow) {
        return Ok(());
    }

    // Collect all known tool names from config
    let known_tools: HashSet<&str> = config
        .tool
        .iter()
        .filter_map(|t| t.name.as_deref())
        .collect();

    let mut unknown_tools = Vec::new();

    for tool_name in skip_tool {
        if !known_tools.contains(tool_name.as_str()) {
            unknown_tools.push(("--skip-tool", tool_name.clone()));
        }
    }

    for tool_name in only_tool {
        if !known_tools.contains(tool_name.as_str()) {
            unknown_tools.push(("--only-tool", tool_name.clone()));
        }
    }

    if unknown_tools.is_empty() {
        return Ok(());
    }

    match level {
        level::Level::Allow => {}
        level::Level::Warn => {
            for (flag, tool_name) in &unknown_tools {
                warn!("unknown tool `{tool_name}` specified in {flag}");
            }
        }
        level::Level::Deny => {
            for (flag, tool_name) in &unknown_tools {
                error!("unknown tool `{tool_name}` specified in {flag}");
            }
            bail!(
                "found unknown tool names and --deny={}",
                Warn::UnknownTool.as_str()
            );
        }
    }

    Ok(())
}

pub(crate) fn check_unlisted_config(lints: &Warns, config: &config::Config) -> anyhow::Result<()> {
    let level = lints.level(Warn::UnlistedConfig);
    if matches!(level, level::Level::Allow) {
        return Ok(());
    }

    let known_tools_by_name = known::known_tools_by_name();
    let mut unlisted_configs = Vec::new();

    for tool in &config.tool {
        if let Some(tool_name) = &tool.name
            && let Some(known_tool) = known_tools_by_name.get(tool_name)
        {
            // Check which configs from the known tool exist
            let existing_known_configs: Vec<&PathBuf> = known_tool
                .configs
                .iter()
                .filter(|config_path| config_path.exists())
                .collect();

            // Check which existing configs are not listed in the tool's configs
            let tool_configs_set: HashSet<&PathBuf> = tool.configs.iter().collect();
            for config_path in existing_known_configs {
                if !tool_configs_set.contains(config_path) {
                    unlisted_configs.push((tool_name.clone(), config_path.clone()));
                }
            }
        }
    }

    if unlisted_configs.is_empty() {
        return Ok(());
    }

    match level {
        level::Level::Allow => {}
        level::Level::Warn => {
            for (tool_name, config_path) in &unlisted_configs {
                warn!(
                    "tool `{tool_name}` has unlisted config file `{}`",
                    config_path.display()
                );
            }
        }
        level::Level::Deny => {
            for (tool_name, config_path) in &unlisted_configs {
                error!(
                    "tool `{tool_name}` has unlisted config file `{}`",
                    config_path.display()
                );
            }
            bail!(
                "found unlisted config files and --deny={}",
                Warn::UnlistedConfig.as_str()
            );
        }
    }

    Ok(())
}

pub(crate) fn check_careful(
    lints: &Warns,
    careful_cli: bool,
    careful_config: bool,
) -> anyhow::Result<()> {
    let level = lints.level(Warn::Careful);
    if matches!(level, level::Level::Allow) {
        return Ok(());
    }

    let careful = careful_cli || careful_config;
    if careful {
        return Ok(());
    }

    match level {
        level::Level::Allow => {}
        level::Level::Warn => {
            warn!("--careful is not set at CLI or config level");
        }
        level::Level::Deny => {
            error!("--careful is not set at CLI or config level");
            bail!("--careful is not set and --deny={}", Warn::Careful.as_str());
        }
    }

    Ok(())
}

pub(crate) fn check_mtime(
    lints: &Warns,
    mtime_cli: bool,
    mtime_config: bool,
) -> anyhow::Result<()> {
    let level = lints.level(Warn::Mtime);
    if matches!(level, level::Level::Allow) {
        return Ok(());
    }

    let mtime = mtime_cli || mtime_config;
    if !mtime {
        return Ok(());
    }

    match level {
        level::Level::Allow => {}
        level::Level::Warn => {
            warn!("mtime is set on CLI or config file");
        }
        level::Level::Deny => {
            error!("mtime is set on CLI or config file");
            bail!("mtime is set and --deny={}", Warn::Mtime.as_str());
        }
    }

    Ok(())
}

pub(crate) fn check_refs(
    lints: &Warns,
    refs_cli: &[String],
    refs_config: &[String],
) -> anyhow::Result<()> {
    let level = lints.level(Warn::Refs);
    if matches!(level, level::Level::Allow) {
        return Ok(());
    }

    let refs_used = !refs_cli.is_empty() || !refs_config.is_empty();
    if !refs_used {
        return Ok(());
    }

    match level {
        level::Level::Allow => {}
        level::Level::Warn => {
            warn!("refs is used on CLI or config file");
        }
        level::Level::Deny => {
            error!("refs is used on CLI or config file");
            bail!("refs is used and --deny={}", Warn::Refs.as_str());
        }
    }

    Ok(())
}
