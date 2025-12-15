use std::path::Path;
use std::{collections::HashSet, fs};

use anyhow::{Context as _, Result};

use crate::cli::Init;
use crate::config::{self, Config};
use crate::known;

fn get_known_tools(names: &[String]) -> Result<(Vec<config::Linter>, Vec<config::Formatter>)> {
    let mut linters = Vec::new();
    let mut formatters = Vec::new();
    for n in names {
        if let Some(mut linter) = known::known_linter_by_name(n) {
            linter.tool.configs.retain(|config| config.exists());
            linters.push(linter);
        } else if let Some(mut formatter) = known::known_formatter_by_name(n) {
            formatter.tool.configs.retain(|config| config.exists());
            formatters.push(formatter);
        } else {
            anyhow::bail!("Unknown tool: {n}");
        }
    }
    Ok((linters, formatters))
}

fn collect_tools(linters: &[String]) -> Result<(Vec<config::Linter>, Vec<config::Formatter>)> {
    if linters.is_empty() {
        let mut detected_linters = Vec::new();
        let mut detected_formatters = Vec::new();
        for mut linter in known::known_linters() {
            linter.tool.configs.retain(|config| config.exists());
            if !linter.tool.configs.is_empty() {
                detected_linters.push(linter);
            }
        }
        for mut formatter in known::known_formatters() {
            formatter.tool.configs.retain(|config| config.exists());
            if !formatter.tool.configs.is_empty() {
                detected_formatters.push(formatter);
            }
        }
        Ok((detected_linters, detected_formatters))
    } else {
        get_known_tools(linters)
    }
}

pub(crate) fn gen_config(init: &Init) -> Result<Config, anyhow::Error> {
    let mut names = HashSet::new();
    let mut tool_names = init.tool.clone();
    tool_names.retain(|l| names.insert(l.clone()));
    let (linter, formatter) = collect_tools(&tool_names)?;
    let config = Config {
        linter,
        formatter,
        refs: init.r#ref.clone(),
        careful: init.careful,
        cores: init.cores,
        mtime: init.mtime,
        ninja: None,
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
            mtime: false,
            r#ref: Vec::new(),
            allow: Vec::new(),
            warn: Vec::new(),
            deny: Vec::new(),
        };
        let config = gen_config(&init).unwrap();
        let toml = toml::to_string_pretty(&config).unwrap();
        expect![[r#"
            [[linter]]
            name = "cargo clippy"
            cmd = "cargo clippy --color=always --all-targets -- --deny warnings"
            files = ["*.rs"]
            granularity = "batch"
            configs = ["Cargo.toml"]
            fix = "cargo clippy --color=always --allow-dirty --fix"

            [[linter]]
            name = "ruff check"
            cmd = "ruff check --"
            files = ["*.py"]
            fix = "ruff check --fix --"
        "#]]
        .assert_eq(&toml);
    }

    #[test]
    fn init_detect() {
        let init = Init {
            tool: Vec::new(),
            careful: false,
            cores: None,
            mtime: false,
            r#ref: Vec::new(),
            allow: Vec::new(),
            warn: Vec::new(),
            deny: Vec::new(),
        };
        let config = gen_config(&init).unwrap();
        let toml = toml::to_string_pretty(&config).unwrap();
        expect![[r#"
            [[linter]]
            name = "cargo clippy"
            cmd = "cargo clippy --color=always --all-targets -- --deny warnings"
            files = ["*.rs"]
            granularity = "batch"
            configs = ["Cargo.toml"]
            fix = "cargo clippy --color=always --allow-dirty --fix"

            [[formatter]]
            name = "cargo fmt"
            cmd = "cargo fmt --"
            files = ["*.rs"]
            granularity = "batch"
            configs = ["Cargo.toml"]
            check = "cargo fmt --check"
        "#]]
        .assert_eq(&toml);
    }
}
