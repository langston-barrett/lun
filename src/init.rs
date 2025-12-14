use std::path::Path;
use std::{collections::HashSet, fs};

use anyhow::{Context as _, Result};

use crate::config::{self, Config};
use crate::known::{self, known_tools_by_name};

fn get_known_tools(names: &[String]) -> std::result::Result<Vec<config::Tool>, anyhow::Error> {
    let mut tools = Vec::with_capacity(names.len());
    let known = known_tools_by_name();
    for n in names {
        if let Some(t) = known.get(n.as_str()) {
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

pub(crate) fn gen_config(mut linters: Vec<String>) -> Result<Config, anyhow::Error> {
    let mut names = HashSet::new();
    linters.retain(|l| names.insert(l.clone()));
    let tool = collect_tools(&linters)?;
    let config = Config {
        tool,
        refs: Vec::new(),
        careful: false,
        cores: None,
        mtime: false,
        ninja: None,
        warns: config::WarnCfg {
            allow: Vec::new(),
            warn: Vec::new(),
            deny: Vec::new(),
        },
    };
    Ok(config)
}

pub(crate) fn go(config_path: &Path, linters: Vec<String>) -> Result<()> {
    let config = gen_config(linters)?;
    let toml = toml::to_string_pretty(&config).context("Failed to serialize config to TOML")?;
    fs::write(config_path, toml)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn init() {
        let config =
            gen_config(vec!["cargo clippy".to_string(), "ruff check".to_string()]).unwrap();
        let toml = toml::to_string_pretty(&config).unwrap();
        expect![[r#"
            [[tool]]
            name = "cargo clippy"
            cmd = "cargo clippy --color=always --all-targets -- --deny warnings"
            files = "*.rs"
            granularity = "batch"
            configs = ["Cargo.toml"]
            fix = "cargo clippy --color=always --allow-dirty --fix"
            formatter = false

            [[tool]]
            name = "ruff check"
            cmd = "ruff check --"
            files = "*.py"
            granularity = "individual"
            configs = [
                "pyproject.toml",
                "ruff.toml",
                ".ruff.toml",
            ]
            fix = "ruff check --fix --"
            formatter = false
        "#]]
        .assert_eq(&toml);
    }

    #[test]
    fn init_detect() {
        let config = gen_config(Vec::new()).unwrap();
        let toml = toml::to_string_pretty(&config).unwrap();
        expect![[r#"
            [[tool]]
            name = "cargo clippy"
            cmd = "cargo clippy --color=always --all-targets -- --deny warnings"
            files = "*.rs"
            granularity = "batch"
            configs = ["Cargo.toml"]
            fix = "cargo clippy --color=always --allow-dirty --fix"
            formatter = false

            [[tool]]
            name = "cargo fmt"
            cmd = "cargo fmt --"
            files = "*.rs"
            granularity = "batch"
            configs = ["Cargo.toml"]
            check = "cargo fmt --check"
            formatter = true
        "#]]
        .assert_eq(&toml);
    }
}
