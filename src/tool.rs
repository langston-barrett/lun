use globset::GlobMatcher;

use crate::{config::Granularity, file::Xxhash, run::RunMode};

#[derive(Clone, Debug)]
pub(crate) struct Tool {
    pub(crate) name: Option<String>,
    pub(crate) cmd: String,
    pub(crate) files: GlobMatcher,
    pub(crate) granularity: Granularity,
    pub(crate) config: Option<Xxhash>,
    pub(crate) check: Option<String>,
    pub(crate) fix: Option<String>,
    pub(crate) formatter: bool,
    pub(crate) version: Option<Xxhash>,
}

impl Tool {
    pub(crate) fn display_name(&self) -> &str {
        self.name.as_ref().unwrap_or(&self.cmd)
    }

    pub(crate) fn get_cmd(&self, mode: RunMode) -> &str {
        match mode {
            RunMode::Fix => {
                if let Some(fix) = &self.fix {
                    fix
                } else {
                    &self.cmd
                }
            }
            RunMode::Check => {
                if self.formatter
                    && let Some(check) = &self.check
                {
                    check
                } else {
                    &self.cmd
                }
            }
            RunMode::Normal => &self.cmd,
        }
    }
}
