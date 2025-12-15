use globset::GlobSet;

use crate::{config::Granularity, file::Xxhash};

/// Hash of command, config file content, and tool version
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Stamp(pub(crate) Xxhash);

#[derive(Clone, Debug)]
pub(crate) struct Tool {
    pub(crate) name: Option<String>,
    pub(crate) cmd: String,
    pub(crate) files: GlobSet,
    pub(crate) ignore: Option<GlobSet>,
    pub(crate) granularity: Granularity,
    pub(crate) stamp: Stamp,
    pub(crate) cd: Option<std::path::PathBuf>,
}

impl Tool {
    pub(crate) fn display_name(&self) -> &str {
        self.name.as_ref().unwrap_or(&self.cmd)
    }
}
