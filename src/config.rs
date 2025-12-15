use std::{
    fs, io,
    io::IsTerminal,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context as _, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use tracing::debug;

use crate::{file, run::RunMode, tool};

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
    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) linter: Vec<Linter>,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) formatter: Vec<Formatter>,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) careful: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) cores: Option<NonZeroUsize>,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) ignore: Vec<String>,

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
    pub(crate) fn load(path: &Path) -> Result<Option<Self>> {
        debug!("Loading config file from {}", path.display());
        let r = fs::read_to_string(path);
        let contents = match r {
            Ok(s) => s,
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => {
                    debug!("No config at {}", path.display());
                    return Ok(None);
                }
                _ => Err(e)
                    .with_context(|| format!("Failed to read config file: {}", path.display()))?,
            },
        };
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

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
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
    pub(crate) cd: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Linter {
    #[serde(flatten)]
    pub(crate) tool: Tool,
    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) fix: Option<String>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Formatter {
    #[serde(flatten)]
    pub(crate) tool: Tool,
    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) check: Option<String>,
}

fn build_tool_stamp(tool: &Tool, cmd: &str, careful: bool) -> Result<tool::Stamp> {
    let tool_name = tool.name.as_ref().unwrap_or(&tool.cmd);
    let config = build_config_hash(tool_name, &tool.configs)?;
    let version = if careful {
        get_tool_version(&tool.cmd).map(|s| file::compute_hash(s.as_bytes()))
    } else {
        None
    };

    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    hasher.update(cmd.as_bytes());
    if let Some(config_hash) = config {
        hasher.update(&config_hash.0.to_le_bytes());
    }
    if let Some(version_hash) = version {
        hasher.update(&version_hash.0.to_le_bytes());
    }
    if let Some(cd) = &tool.cd {
        hasher.update(cd.as_os_str().as_encoded_bytes());
    }
    Ok(tool::Stamp(file::Xxhash(hasher.digest())))
}

fn build_tool_globsets(
    tool: &Tool,
    global_ignore: &[String],
) -> Result<(GlobSet, Option<GlobSet>)> {
    let tool_name = tool.name.as_ref().unwrap_or(&tool.cmd);
    let files = build_files_globset(&tool.files, tool_name)?;
    let mut all_ignore = global_ignore.to_vec();
    all_ignore.extend_from_slice(&tool.ignore);
    let ignore = build_ignore_globset(&all_ignore, tool_name)?;
    Ok((files, ignore))
}

impl Linter {
    pub(crate) fn into_tool(
        self,
        mode: RunMode,
        careful: bool,
        color: crate::cli::log::Color,
        global_ignore: &[String],
    ) -> Result<tool::Tool> {
        let color_str = color_to_str(color);
        let cmd = match mode {
            RunMode::Fix => {
                if let Some(fix) = &self.fix {
                    fix.replace("{{color}}", color_str)
                } else {
                    self.tool.cmd.replace("{{color}}", color_str)
                }
            }
            RunMode::Check | RunMode::Normal => self.tool.cmd.replace("{{color}}", color_str),
        };

        let (files, ignore) = build_tool_globsets(&self.tool, global_ignore)?;
        let stamp = build_tool_stamp(&self.tool, &cmd, careful)?;

        Ok(tool::Tool {
            name: self.tool.name,
            cmd,
            files,
            ignore,
            granularity: self.tool.granularity,
            stamp,
            cd: self.tool.cd,
        })
    }
}

impl Formatter {
    pub(crate) fn into_tool(
        self,
        mode: RunMode,
        careful: bool,
        color: crate::cli::log::Color,
        global_ignore: &[String],
    ) -> Result<tool::Tool> {
        let color_str = color_to_str(color);
        let cmd = match mode {
            RunMode::Check => {
                if let Some(check) = &self.check {
                    check.replace("{{color}}", color_str)
                } else {
                    self.tool.cmd.replace("{{color}}", color_str)
                }
            }
            RunMode::Fix | RunMode::Normal => self.tool.cmd.replace("{{color}}", color_str),
        };

        let (files, ignore) = build_tool_globsets(&self.tool, global_ignore)?;
        let stamp = build_tool_stamp(&self.tool, &cmd, careful)?;

        Ok(tool::Tool {
            name: self.tool.name,
            cmd,
            files,
            ignore,
            granularity: self.tool.granularity,
            stamp,
            cd: self.tool.cd,
        })
    }
}

fn color_to_str(color: crate::cli::log::Color) -> &'static str {
    match color {
        crate::cli::log::Color::Always => "always",
        crate::cli::log::Color::Never => "never",
        crate::cli::log::Color::Auto => {
            if io::stdout().is_terminal() {
                "always"
            } else {
                "never"
            }
        }
    }
}

fn build_config_hash(tool: &str, configs: &[PathBuf]) -> Result<Option<file::Xxhash>> {
    if configs.is_empty() {
        return Ok(None);
    }
    let mut sorted = configs.to_vec();
    sorted.sort();
    let mut combined = Vec::new();
    for path in &sorted {
        let content = fs::read_to_string(path).with_context(|| {
            format!(
                "Failed to read config file for `{tool}`: {}",
                path.display()
            )
        })?;
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
