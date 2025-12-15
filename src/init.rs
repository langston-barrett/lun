use std::path::Path;
use std::{collections::HashSet, fs};

use anyhow::{Context as _, Result};

use crate::cli::Init;
use crate::config::{self, Config};
use crate::known::{self, known_tools_by_name};

fn get_known_tools(names: &[String]) -> std::result::Result<Vec<config::Tool>, anyhow::Error> {
    let mut tools = Vec::with_capacity(names.len());
    let known = known_tools_by_name();
    for n in names {
        if let Some(t) = known.get(n.as_str()) {
            let mut t = t.clone();
            t.configs.retain(|config| config.exists());
            tools.push(t.clone());
        } else {
            anyhow::bail!("Unknown tool: {n}");
        }
    }
    Ok(tools)
}

fn collect_tools(linters: &[String]) -> Result<Vec<config::Tool>> {
    if linters.is_empty() {
        Ok(known::detect())
    } else {
        get_known_tools(linters)
    }
}

pub(crate) fn gen_config(init: &Init) -> Result<Config, anyhow::Error> {
    let mut names = HashSet::new();
    let mut linters = init.tool.clone();
    linters.retain(|l| names.insert(l.clone()));
    let tool = collect_tools(&linters)?;
    let config = Config {
        tool,
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
            [[tool]]
            name = "cargo clippy"
            cmd = "cargo clippy --color=always --all-targets -- --deny warnings"
            files = ["*.rs"]
            granularity = "batch"
            configs = ["Cargo.toml"]
            fix = "cargo clippy --color=always --allow-dirty --fix"

            [[tool]]
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
            [[tool]]
            name = "cargo clippy"
            cmd = "cargo clippy --color=always --all-targets -- --deny warnings"
            files = ["*.rs"]
            granularity = "batch"
            configs = ["Cargo.toml"]
            fix = "cargo clippy --color=always --allow-dirty --fix"

            [[tool]]
            name = "cargo fmt"
            cmd = "cargo fmt --"
            files = ["*.rs"]
            granularity = "batch"
            configs = ["Cargo.toml"]
            check = "cargo fmt --check"
            formatter = true
        "#]]
        .assert_eq(&toml);
    }
}
