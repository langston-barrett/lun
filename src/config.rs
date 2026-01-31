use std::{
    env, fs, io,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context as _, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use tracing::debug;

use crate::{file, git, known, out, run::RunMode, tool};

pub(crate) const CONFIG_FILE_NAME: &str = "lun.toml";

fn default<T: Default + PartialEq>(t: &T) -> bool {
    *t == Default::default()
}

fn default_mtime() -> bool {
    true
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_default_mtime(mtime: &bool) -> bool {
    *mtime == default_mtime()
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

/// Default ignore patterns for image files
pub(crate) const DEFAULT_IGNORES: &[&str] = &[
    "*.jpg", "*.jpeg", "*.JPG", "*.JPEG", "*.png", "*.PNG", "*.svg", "*.SVG",
];

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
    pub(crate) cache_size: Option<usize>,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) careful: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) cores: Option<NonZeroUsize>,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) ignore: Vec<String>,

    #[serde(default = "default_mtime")]
    #[serde(skip_serializing_if = "is_default_mtime")]
    pub(crate) mtime: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) ninja: Option<bool>,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) no_default_ignores: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) refs: Vec<git::Ref>,

    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) tool: Vec<KnownTool>,

    #[serde(flatten)]
    pub(crate) warns: WarnCfg,
}

impl Config {
    /// Search for a config file starting from the current directory and walking up.
    pub(crate) fn find() -> Option<PathBuf> {
        let mut current = env::current_dir().ok()?;
        loop {
            let config_path = current.join(CONFIG_FILE_NAME);
            if config_path.is_file() {
                debug!("Found config at {}", config_path.display());
                return Some(config_path);
            }
            if !current.pop() {
                debug!("No config found in any parent directory");
                return None;
            }
        }
    }

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
        let mut config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        config.known_tools()?;
        Ok(Some(config))
    }

    pub(crate) fn known_tools(&mut self) -> Result<()> {
        for known_tool in &self.tool {
            if let Some(mut linter) = known::known_linter_by_name(&known_tool.name) {
                known_tool.merge_into_linter(&mut linter);
                self.linter.push(linter);
            } else if let Some(mut formatter) = known::known_formatter_by_name(&known_tool.name) {
                known_tool.merge_into_formatter(&mut formatter);
                self.formatter.push(formatter);
            } else {
                anyhow::bail!("Unknown tool name in [[tool]]: {}", known_tool.name);
            }
        }
        Ok(())
    }

    /// Returns the effective ignore patterns, combining default ignores with user ignores.
    pub(crate) fn effective_ignore(&self) -> Vec<String> {
        if self.no_default_ignores {
            self.ignore.clone()
        } else {
            let mut ignores: Vec<String> =
                DEFAULT_IGNORES.iter().map(|s| (*s).to_string()).collect();
            ignores.extend(self.ignore.clone());
            ignores
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Args {
    None,
    One,
    #[default]
    Many,
    All,
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
    pub(crate) args: Args,
    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) include_unchanged: bool,
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

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct KnownTool {
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) cmd: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) files: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "default")]
    pub(crate) ignore: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) args: Option<Args>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) include_unchanged: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) configs: Option<Vec<PathBuf>>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) cd: Option<PathBuf>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) fix: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) check: Option<String>,
}

impl KnownTool {
    fn merge_into_linter(&self, linter: &mut Linter) {
        if let Some(ref cmd) = self.cmd {
            linter.tool.cmd.clone_from(cmd);
        }
        if !self.files.is_empty() {
            linter.tool.files.clone_from(&self.files);
        }
        if !self.ignore.is_empty() {
            linter.tool.ignore.clone_from(&self.ignore);
        }
        if let Some(args) = self.args {
            linter.tool.args = args;
        }
        if let Some(include_unchanged) = self.include_unchanged {
            linter.tool.include_unchanged = include_unchanged;
        }
        if let Some(ref configs) = self.configs {
            linter.tool.configs.clone_from(configs);
        }
        if let Some(ref cd) = self.cd {
            linter.tool.cd = Some(cd.clone());
        }
        if let Some(ref fix) = self.fix {
            linter.fix = Some(fix.clone());
        }
    }

    fn merge_into_formatter(&self, formatter: &mut Formatter) {
        if let Some(ref cmd) = self.cmd {
            formatter.tool.cmd.clone_from(cmd);
        }
        if !self.files.is_empty() {
            formatter.tool.files.clone_from(&self.files);
        }
        if !self.ignore.is_empty() {
            formatter.tool.ignore.clone_from(&self.ignore);
        }
        if let Some(args) = self.args {
            formatter.tool.args = args;
        }
        if let Some(include_unchanged) = self.include_unchanged {
            formatter.tool.include_unchanged = include_unchanged;
        }
        if let Some(ref configs) = self.configs {
            formatter.tool.configs.clone_from(configs);
        }
        if let Some(ref cd) = self.cd {
            formatter.tool.cd = Some(cd.clone());
        }
        if let Some(ref check) = self.check {
            formatter.check = Some(check.clone());
        }
    }
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

    let exe_name = cmd.split_whitespace().next().unwrap_or(cmd);
    let env_pfx = format!("{}_", exe_name.to_uppercase());
    let mut env_vars = env::vars_os()
        .filter(|(key, _)| key.as_encoded_bytes().starts_with(env_pfx.as_bytes()))
        .collect::<Vec<_>>();
    env_vars.sort_by(|a, b| a.0.cmp(&b.0));
    for (key, value) in &env_vars {
        debug!("Found relevant environment variable {}", key.display());
        hasher.update(key.as_encoded_bytes());
        hasher.update(value.as_encoded_bytes());
    }

    Ok(tool::Stamp(file::Xxhash(hasher.digest128())))
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
        out_config: out::Config,
        global_ignore: &[String],
    ) -> Result<tool::Tool> {
        let color_str = out_config.color_as_str();
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
            args: self.tool.args,
            include_unchanged: self.tool.include_unchanged,
            stamp,
            cd: self.tool.cd,
            configs: self.tool.configs,
        })
    }
}

impl Formatter {
    pub(crate) fn into_tool(
        self,
        mode: RunMode,
        careful: bool,
        out_config: out::Config,
        global_ignore: &[String],
    ) -> Result<tool::Tool> {
        let color_str = out_config.color_as_str();
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
            args: self.tool.args,
            include_unchanged: self.tool.include_unchanged,
            stamp,
            cd: self.tool.cd,
            configs: self.tool.configs,
        })
    }
}

fn build_config_hash(tool: &str, configs: &[PathBuf]) -> Result<Option<file::Xxhash>> {
    if configs.is_empty() {
        return Ok(None);
    }
    let mut sorted = configs.to_vec();
    sorted.sort();
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    for path in &sorted {
        let metadata = fs::metadata(path).with_context(|| {
            format!(
                "Failed to get metadata for config file for `{tool}`: {}",
                path.display()
            )
        })?;
        file::hash_md(path, &metadata, &mut hasher);
        file::hash_mtime(path, &metadata, &mut hasher)?;
    }
    Ok(Some(file::Xxhash(hasher.digest128())))
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
        let normalized = pattern.strip_prefix("./").unwrap_or(pattern);
        let glob = Glob::new(normalized)
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
        let version_output = if output.stdout.is_empty() {
            String::from_utf8_lossy(&output.stderr)
        } else {
            String::from_utf8_lossy(&output.stdout)
        };
        let version = version_output.trim();
        debug!("Tool {} version: {}", program, version);
        Some(version.to_string())
    } else {
        debug!("Failed to get version for {}: {}", program, output.status);
        None
    }
}
