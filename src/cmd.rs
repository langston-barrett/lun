use std::{process, sync::Arc};

use crate::{config, file, run::RunMode, tool};

#[derive(Debug, Clone)]
pub(crate) struct Command {
    pub(crate) tool: Arc<tool::Tool>,
    pub(crate) files: Vec<file::File>,
}

impl Command {
    pub(crate) fn to_command(&self, mode: RunMode) -> process::Command {
        let cmd_str = self.tool.get_cmd(mode);
        let parts: Vec<String> = cmd_str.split_whitespace().map(|s| s.to_string()).collect();
        let mut cmd = process::Command::new(&parts[0]);
        cmd.args(&parts[1..]);
        if self.tool.granularity == config::Granularity::Individual {
            for f in &self.files {
                cmd.arg(f.path.as_path());
            }
        }
        cmd
    }
}
