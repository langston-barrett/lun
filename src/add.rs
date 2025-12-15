use std::{fs, io::Write as _, path::Path};

use anyhow::{Context as _, Result};

use crate::{cli, known};

fn gen_tool(options: &cli::Add) -> Result<String, anyhow::Error> {
    let is_formatter = if let Some(formatter) = options.formatter {
        formatter
    } else {
        // Try to determine from known tools
        known::known_formatter_by_name(&options.tool).is_some()
    };

    if is_formatter {
        let mut formatter = known::known_formatter_by_name(&options.tool)
            .ok_or_else(|| anyhow::anyhow!("Unknown formatter: {}", options.tool))?;
        if let Some(name) = &options.name {
            formatter.tool.name = Some(name.clone());
        }
        if let Some(cmd) = &options.cmd {
            formatter.tool.cmd = cmd.clone();
        }
        if let Some(files) = &options.files {
            formatter.tool.files = vec![files.clone()];
        }
        if let Some(check) = &options.check {
            formatter.check = Some(check.clone());
        }
        if let Some(config_path) = &options.config {
            formatter.tool.configs = vec![config_path.clone()];
        }
        let toml =
            toml::to_string_pretty(&formatter).context("Failed to serialize formatter to TOML")?;
        Ok(format!("[[formatter]]\n{toml}"))
    } else {
        let mut linter = known::known_linter_by_name(&options.tool)
            .ok_or_else(|| anyhow::anyhow!("Unknown linter: {}", options.tool))?;
        if let Some(name) = &options.name {
            linter.tool.name = Some(name.clone());
        }
        if let Some(cmd) = &options.cmd {
            linter.tool.cmd = cmd.clone();
        }
        if let Some(files) = &options.files {
            linter.tool.files = vec![files.clone()];
        }
        if let Some(config_path) = &options.config {
            linter.tool.configs = vec![config_path.clone()];
        }
        let toml = toml::to_string_pretty(&linter).context("Failed to serialize linter to TOML")?;
        Ok(format!("[[linter]]\n{toml}"))
    }
}

pub(crate) fn go(config_path: &Path, options: &cli::Add) -> Result<()> {
    let toml = gen_tool(options)?;
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(config_path)
        .with_context(|| format!("Couldn't append to config at {}", config_path.display()))?;
    writeln!(file)?;
    writeln!(file, "{toml}")?;
    writeln!(file)?;
    Ok(())
}
