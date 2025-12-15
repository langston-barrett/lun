use std::{
    fs,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context as _, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use tracing::debug;

use crate::{file, tool};

fn default<T: Default + PartialEq>(t: &T) -> bool {
    *t == Default::default()
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WarnCfg {
    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) allow: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) warn: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) deny: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    pub(crate) tool: Vec<Tool>,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) careful: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) cores: Option<NonZeroUsize>,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) mtime: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) ninja: Option<bool>,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) refs: Vec<String>,

    #[serde(flatten)]
    pub(crate) warns: WarnCfg,
}

impl Config {
    pub(crate) fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Granularity {
    #[default]
    Individual,
    Batch,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Tool {
    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) name: Option<String>,
    pub(crate) cmd: String,
    pub(crate) files: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) ignore: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) granularity: Granularity,
    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) configs: Vec<PathBuf>,
    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) check: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) fix: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) formatter: bool,
}

impl Tool {
    pub(crate) fn into_tool(self, careful: bool) -> Result<tool::Tool> {
        let config = build_config_hash(&self.configs)?;
        let tool_name = self.name.as_ref().unwrap_or(&self.cmd);
        let files = build_files_globset(&self.files, tool_name)?;
        let ignore = build_ignore_globset(&self.ignore, tool_name)?;

        let version = if careful {
            get_tool_version(&self.cmd).map(|s| file::compute_hash(s.as_bytes()))
        } else {
            None
        };

        Ok(tool::Tool {
            name: self.name,
            cmd: self.cmd,
            files,
            ignore,
            granularity: self.granularity,
            config,
            check: self.check,
            fix: self.fix,
            formatter: self.formatter,
            version,
        })
    }
}

fn build_config_hash(configs: &[PathBuf]) -> Result<Option<file::Xxhash>> {
    if configs.is_empty() {
        return Ok(None);
    }
    let mut sorted = configs.to_vec();
    sorted.sort();
    let mut combined = Vec::new();
    for path in &sorted {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        combined.extend_from_slice(path.as_os_str().as_encoded_bytes());
        combined.extend_from_slice(content.as_bytes());
    }
    Ok(Some(file::compute_hash(&combined)))
}

fn build_files_globset(patterns: &[String], tool_name: &str) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern)
            .with_context(|| format!("Invalid `files` glob `{pattern}` for `{tool_name}`"))?;
        builder.add(glob);
    }
    builder
        .build()
        .with_context(|| format!("Failed to build `files` glob set for `{tool_name}`"))
}

fn build_ignore_globset(patterns: &[String], tool_name: &str) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern)
            .with_context(|| format!("Invalid `ignore` glob `{pattern}` for `{tool_name}`"))?;
        builder.add(glob);
    }
    builder
        .build()
        .with_context(|| format!("Failed to build `ignore` glob set for `{tool_name}`"))
        .map(Some)
}

fn get_tool_version(cmd: &str) -> Option<String> {
    let program = cmd.split_whitespace().next()?;
    let output = process::Command::new(program)
        .arg("--version")
        .output()
        .ok()?;
    if output.status.success() {
        let version_output = if !output.stdout.is_empty() {
            String::from_utf8_lossy(&output.stdout)
        } else {
            String::from_utf8_lossy(&output.stderr)
        };
        let version = version_output.trim();
        debug!("Tool {} version: {}", program, version);
        Some(version.to_string())
    } else {
        debug!("Failed to get version for {}: {}", program, output.status);
        None
    }
}
