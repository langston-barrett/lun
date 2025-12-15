use std::{fs, io::Write as _, path::Path};

use anyhow::{Context as _, Result};

use crate::{cli, config, known};

fn gen_tool(options: &cli::Add) -> Result<config::Tool, anyhow::Error> {
    let known_tools = known::known_tools_by_name();
    let base_tool = known_tools
        .get(&options.tool)
        .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", options.tool))?;
    let mut tool = base_tool.clone();
    if let Some(name) = &options.name {
        tool.name = Some(name.clone());
    }
    if let Some(formatter) = options.formatter {
        tool.formatter = formatter;
    }
    if let Some(cmd) = &options.cmd {
        tool.cmd = cmd.clone();
    }
    if let Some(files) = &options.files {
        tool.files = vec![files.clone()];
    }
    if let Some(check) = &options.check {
        tool.check = Some(check.clone());
    }
    if let Some(config_path) = &options.config {
        tool.configs = vec![config_path.clone()];
    }
    Ok(tool)
}

pub(crate) fn go(config_path: &Path, options: &cli::Add) -> Result<()> {
    let tool = gen_tool(options)?;
    let toml = toml::to_string_pretty(&tool).context("Failed to serialize tool to TOML")?;
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(config_path)
        .with_context(|| format!("Couldn't append to config at {}", config_path.display()))?;
    writeln!(file)?;
    writeln!(file, "{toml}")?;
    writeln!(file)?;
    Ok(())
}
