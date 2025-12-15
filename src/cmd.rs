use std::{process, sync::Arc};

use crate::{config, file, tool};

#[derive(Debug, Clone)]
pub(crate) struct Command {
    pub(crate) tool: Arc<tool::Tool>,
    pub(crate) files: Vec<file::File>,
}

impl Command {
    pub(crate) fn to_command(&self) -> process::Command {
        let cmd_str = &self.tool.cmd;
        let parts: Vec<String> = cmd_str.split_whitespace().map(|s| s.to_string()).collect();
        let mut cmd = process::Command::new(&parts[0]);
        cmd.args(&parts[1..]);
        if let Some(cd) = &self.tool.cd {
            cmd.current_dir(cd);
        }
        if self.tool.granularity == config::Granularity::Individual {
            for f in &self.files {
                let path = if let Some(cd) = &self.tool.cd {
                    f.path.strip_prefix(cd).unwrap_or(f.path.as_path())
                } else {
                    f.path.as_path()
                };
                cmd.arg(path);
            }
        }
        cmd
    }
}
