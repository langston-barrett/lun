use std::{collections::HashMap, path::PathBuf};

use crate::config::{self, Granularity};

pub(crate) fn known_linters() -> Vec<config::Linter> {
    vec![
        config::Linter {
            tool: config::Tool {
                name: Some(String::from("cargo clippy")),
                cmd: "cargo clippy --color={{color}} --all-targets -- --deny warnings".to_string(),
                files: vec!["*.rs".to_string()],
                ignore: Vec::new(),
                granularity: Granularity::Batch,
                configs: vec![PathBuf::from("Cargo.toml")],
            },
            fix: Some("cargo clippy --color={{color}} --allow-dirty --fix".to_string()),
        },
        config::Linter {
            tool: config::Tool {
                name: None,
                cmd: "hlint --".to_string(),
                files: vec!["*.hs".to_string()],
                ignore: Vec::new(),
                granularity: Granularity::Individual,
                configs: vec![PathBuf::from(".hlint.yml"), PathBuf::from(".hlint.yaml")],
            },
            fix: None,
        },
        config::Linter {
            tool: config::Tool {
                name: Some(String::from("mdlynx")),
                cmd: "mdlynx --".to_string(),
                files: vec!["*.md".to_string()],
                ignore: Vec::new(),
                granularity: Granularity::Individual,
                configs: Vec::new(),
            },
            fix: None,
        },
        config::Linter {
            tool: config::Tool {
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
            },
            fix: None,
        },
        config::Linter {
            tool: config::Tool {
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
            },
            fix: Some("ruff check --fix --".to_string()),
        },
        config::Linter {
            tool: config::Tool {
                name: Some("shellcheck".to_string()),
                cmd: "shellcheck --color={{color}} --".to_string(),
                files: vec!["*.sh".to_string()],
                ignore: Vec::new(),
                granularity: Granularity::Individual,
                configs: vec![PathBuf::from(".shellcheckrc")],
            },
            fix: None,
        },
        config::Linter {
            tool: config::Tool {
                name: Some("ty".to_string()),
                cmd: "ty check --".to_string(),
                files: vec!["*.py".to_string()],
                ignore: Vec::new(),
                granularity: Granularity::Batch,
                configs: vec![PathBuf::from("pyproject.toml"), PathBuf::from("ty.toml")],
            },
            fix: None,
        },
        config::Linter {
            tool: config::Tool {
                name: Some("ttlint".to_string()),
                cmd: "ttlint --".to_string(),
                files: vec!["*".to_string()],
                ignore: Vec::new(),
                granularity: Granularity::Individual,
                configs: Vec::new(),
            },
            fix: Some("ttlint --fix --".to_string()),
        },
        config::Linter {
            tool: config::Tool {
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
            },
            fix: Some("typos --write-changes --".to_string()),
        },
        config::Linter {
            tool: config::Tool {
                name: Some("zizmor".to_string()),
                cmd: "zizmor --".to_string(),
                files: vec![".github/**/*.yml".to_string()],
                ignore: Vec::new(),
                granularity: Granularity::Individual,
                configs: vec![PathBuf::from("zizmor.yml"), PathBuf::from("zizmor.yaml")],
            },
            fix: Some("zizmor --fix=safe --".to_string()),
        },
    ]
}

pub(crate) fn known_formatters() -> Vec<config::Formatter> {
    vec![
        config::Formatter {
            tool: config::Tool {
                name: Some(String::from("cargo fmt")),
                cmd: "cargo fmt -- --color={{color}} --".to_string(),
                files: vec!["*.rs".to_string()],
                ignore: Vec::new(),
                // This is usually faster as a batch, Cargo is magic
                granularity: Granularity::Batch,
                configs: vec![
                    PathBuf::from("Cargo.toml"),
                    PathBuf::from("rustfmt.toml"),
                    PathBuf::from(".rustfmt.toml"),
                ],
            },
            check: Some("cargo fmt --check -- --color={{color}} --".to_string()),
        },
        config::Formatter {
            tool: config::Tool {
                name: Some("ruff format".to_string()),
                cmd: "ruff format --".to_string(),
                files: vec!["*.py".to_string()],
                ignore: Vec::new(),
                granularity: Granularity::Individual,
                configs: vec![PathBuf::from("ruff.toml"), PathBuf::from(".ruff.toml")],
            },
            check: Some("ruff format --check --".to_string()),
        },
        config::Formatter {
            tool: config::Tool {
                name: Some("taplo".to_string()),
                cmd: "taplo format --".to_string(),
                files: vec!["*.toml".to_string()],
                ignore: Vec::new(),
                granularity: Granularity::Individual,
                configs: vec![],
            },
            check: Some("taplo format --check --".to_string()),
        },
    ]
}

pub(crate) fn known_tools_by_name() -> HashMap<String, config::Tool> {
    let mut m = HashMap::new();
    for linter in known_linters() {
        if let Some(name) = &linter.tool.name {
            debug_assert!(!m.contains_key(name));
            m.insert(name.clone(), linter.tool);
        }
    }
    for formatter in known_formatters() {
        if let Some(name) = &formatter.tool.name {
            debug_assert!(!m.contains_key(name));
            m.insert(name.clone(), formatter.tool);
        }
    }
    m
}

pub(crate) fn known_linter_by_name(name: &str) -> Option<config::Linter> {
    known_linters()
        .into_iter()
        .find(|l| l.tool.name.as_deref() == Some(name))
}

pub(crate) fn known_formatter_by_name(name: &str) -> Option<config::Formatter> {
    known_formatters()
        .into_iter()
        .find(|f| f.tool.name.as_deref() == Some(name))
}
