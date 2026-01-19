use std::path::Path;
use std::{collections::HashSet, fs};

use anyhow::{Context as _, Result};

use crate::cli::Init;
use crate::config::{self, Config};
use crate::known;

fn get_known_tools(names: &[String]) -> Result<Vec<config::KnownTool>> {
    let mut tools = Vec::new();
    for n in names {
        if let Some(linter) = known::known_linter_by_name(n) {
            let mut configs = linter.tool.configs;
            configs.retain(|config| config.exists());
            tools.push(config::KnownTool {
                name: n.clone(),
                configs,
                ..Default::default()
            });
        } else if let Some(formatter) = known::known_formatter_by_name(n) {
            let mut configs = formatter.tool.configs;
            configs.retain(|config| config.exists());
            tools.push(config::KnownTool {
                name: n.clone(),
                configs,
                ..Default::default()
            });
        } else {
            anyhow::bail!("Unknown tool: {n}");
        }
    }
    Ok(tools)
}

fn collect_tools(tool_names: &[String]) -> Result<Vec<config::KnownTool>> {
    if tool_names.is_empty() {
        let mut tools = Vec::new();
        for linter in known::known_linters() {
            let Some(name) = linter.tool.name else {
                continue;
            };
            let mut configs = linter.tool.configs;
            configs.retain(|config| config.exists());
            if !configs.is_empty() {
                tools.push(config::KnownTool {
                    name,
                    configs,
                    ..Default::default()
                });
            }
        }
        for formatter in known::known_formatters() {
            let Some(name) = formatter.tool.name else {
                continue;
            };
            let mut configs = formatter.tool.configs;
            configs.retain(|config| config.exists());
            if !configs.is_empty() {
                tools.push(config::KnownTool {
                    name,
                    configs,
                    ..Default::default()
                });
            }
        }
        Ok(tools)
    } else {
        get_known_tools(tool_names)
    }
}

pub(crate) fn gen_config(init: &Init) -> Result<Config, anyhow::Error> {
    let mut names = HashSet::new();
    let mut tool_names = init.tool.clone();
    tool_names.retain(|l| names.insert(l.clone()));
    let tool = collect_tools(&tool_names)?;
    let config = Config {
        linter: Vec::new(),
        formatter: Vec::new(),
        refs: init.r#ref.clone(),
        careful: init.careful,
        cores: init.cores,
        mtime: !init.no_mtime,
        ninja: None,
        ignore: Vec::new(),
        cache_size: None,
        tool,
        warns: config::WarnCfg {
            allow: init.allow.clone(),
            warn: init.warn.clone(),
            deny: init.deny.clone(),
        },
    };
    Ok(config)
}

pub(crate) fn go(config_path: &Path, init: &Init) -> Result<()> {
    let config = gen_config(init)?;
    let toml = toml::to_string_pretty(&config).context("Failed to serialize config to TOML")?;
    let mut s = String::from("# https://langston-barrett.github.io/lun/config.html\n\n");
    s.push_str(&toml);
    fs::write(config_path, s)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn init() {
        let init = Init {
            tool: vec!["cargo clippy".to_string(), "ruff check".to_string()],
            careful: false,
            cores: None,
            no_mtime: false,
            r#ref: Vec::new(),
            allow: Vec::new(),
            warn: Vec::new(),
            deny: Vec::new(),
        };
        let config = gen_config(&init).unwrap();
        let toml = toml::to_string_pretty(&config).unwrap();
        expect![[r#"
            [[tool]]
            name = "cargo clippy"
            configs = ["Cargo.toml"]

            [[tool]]
            name = "ruff check"
        "#]]
        .assert_eq(&toml);
    }

    #[test]
    fn init_detect() {
        let init = Init {
            tool: Vec::new(),
            careful: false,
            cores: None,
            no_mtime: false,
            r#ref: Vec::new(),
            allow: Vec::new(),
            warn: Vec::new(),
            deny: Vec::new(),
        };
        let config = gen_config(&init).unwrap();
        let toml = toml::to_string_pretty(&config).unwrap();
        expect![[r#"
            [[tool]]
            name = "cargo clippy"
            configs = ["Cargo.toml"]

            [[tool]]
            name = "cargo fmt"
            configs = ["Cargo.toml"]
        "#]]
        .assert_eq(&toml);
    }
}
