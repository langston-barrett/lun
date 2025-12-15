use std::{collections::HashMap, path::PathBuf};

use crate::config::{self, Granularity};

pub(crate) fn known_tools() -> Vec<config::Tool> {
    vec![
        config::Tool {
            name: Some(String::from("cargo clippy")),
            cmd: "cargo clippy --color=always --all-targets -- --deny warnings".to_string(),
            files: vec!["*.rs".to_string()],
            ignore: Vec::new(),
            granularity: Granularity::Batch,
            configs: vec![PathBuf::from("Cargo.toml")],
            check: None,
            fix: Some("cargo clippy --color=always --allow-dirty --fix".to_string()),
            formatter: false,
        },
        config::Tool {
            name: Some(String::from("cargo fmt")),
            cmd: "cargo fmt --".to_string(),
            files: vec!["*.rs".to_string()],
            ignore: Vec::new(),
            // This is usually faster as a batch, Cargo is magic
            granularity: Granularity::Batch,
            configs: vec![
                PathBuf::from("Cargo.toml"),
                PathBuf::from("rustfmt.toml"),
                PathBuf::from(".rustfmt.toml"),
            ],
            check: Some("cargo fmt --check".to_string()),
            fix: None,
            formatter: true,
        },
        config::Tool {
            name: None,
            cmd: "hlint --".to_string(),
            files: vec!["*.hs".to_string()],
            ignore: Vec::new(),
            granularity: Granularity::Individual,
            configs: vec![PathBuf::from(".hlint.yml"), PathBuf::from(".hlint.yaml")],
            check: None,
            fix: None,
            formatter: false,
        },
        config::Tool {
            name: Some(String::from("mdlynx")),
            cmd: "mdlynx --".to_string(),
            files: vec!["*.md".to_string()],
            ignore: Vec::new(),
            granularity: Granularity::Individual,
            configs: Vec::new(),
            check: None,
            fix: None,
            formatter: false,
        },
        config::Tool {
            name: Some(String::from("mypy")),
            cmd: "mypy --strict --".to_string(),
            files: vec!["*.py".to_string()],
            ignore: Vec::new(),
            granularity: Granularity::Individual,
            configs: vec![
                PathBuf::from("pyproject.toml"),
                PathBuf::from("mypy.ini"),
                PathBuf::from(".mypy.ini"),
            ],
            check: None,
            fix: None,
            formatter: false,
        },
        config::Tool {
            name: Some(String::from("ruff check")),
            cmd: "ruff check --".to_string(),
            files: vec!["*.py".to_string()],
            ignore: Vec::new(),
            granularity: Granularity::Individual,
            configs: vec![
                PathBuf::from("pyproject.toml"),
                PathBuf::from("ruff.toml"),
                PathBuf::from(".ruff.toml"),
            ],
            check: None,
            fix: Some("ruff check --fix --".to_string()),
            formatter: false,
        },
        config::Tool {
            name: Some("ruff format".to_string()),
            cmd: "ruff format --".to_string(),
            files: vec!["*.py".to_string()],
            ignore: Vec::new(),
            granularity: Granularity::Individual,
            configs: vec![PathBuf::from("ruff.toml"), PathBuf::from(".ruff.toml")],
            check: Some("ruff format --check --".to_string()),
            fix: None,
            formatter: true,
        },
        config::Tool {
            name: Some("shellcheck".to_string()),
            cmd: "shellcheck --color=always --".to_string(),
            files: vec!["*.sh".to_string()],
            ignore: Vec::new(),
            granularity: Granularity::Individual,
            configs: vec![PathBuf::from(".shellcheckrc")],
            check: None,
            fix: None,
            formatter: false,
        },
        config::Tool {
            name: Some("ty".to_string()),
            cmd: "ty check --".to_string(),
            files: vec!["*.py".to_string()],
            ignore: Vec::new(),
            granularity: Granularity::Batch,
            configs: vec![PathBuf::from("pyproject.toml"), PathBuf::from("ty.toml")],
            check: None,
            fix: None,
            formatter: false,
        },
        config::Tool {
            name: Some("ttlint".to_string()),
            cmd: "ttlint --".to_string(),
            files: vec!["*".to_string()],
            ignore: Vec::new(),
            granularity: Granularity::Individual,
            configs: Vec::new(),
            check: None,
            fix: Some("ttlint --fix --".to_string()),
            formatter: false,
        },
        config::Tool {
            name: Some("typos".to_string()),
            cmd: "typos --".to_string(),
            files: vec!["*.md".to_string()],
            ignore: Vec::new(),
            granularity: Granularity::Individual,
            configs: vec![
                PathBuf::from("typos.toml"),
                PathBuf::from("_typos.toml"),
                PathBuf::from(".typos.toml"),
            ],
            check: None,
            fix: Some("typos --write-changes --".to_string()),
            formatter: false,
        },
        config::Tool {
            name: Some("zizmor".to_string()),
            cmd: "zizmor --".to_string(),
            files: vec![".github/**/*.yml".to_string()],
            ignore: Vec::new(),
            granularity: Granularity::Individual,
            configs: vec![PathBuf::from("zizmor.yml"), PathBuf::from("zizmor.yaml")],
            check: None,
            fix: Some("zizmor --fix=safe --".to_string()),
            formatter: false,
        },
    ]
}

pub(crate) fn known_tools_by_name() -> HashMap<String, config::Tool> {
    let known = known_tools();
    let mut m = HashMap::with_capacity(known.len());
    for tool in known {
        if let Some(name) = &tool.name {
            debug_assert!(!m.contains_key(name));
            m.insert(name.clone(), tool);
        }
    }
    m
}

pub(crate) fn detect() -> Vec<config::Tool> {
    let mut detected = Vec::new();
    for mut tool in known_tools() {
        tool.configs.retain(|config| config.exists());
        if !tool.configs.is_empty() {
            detected.push(tool);
        }
    }
    detected
}
