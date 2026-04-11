use std::path::PathBuf;

use globset::GlobSet;

use crate::{config::Args, file::Xxhash};

/// Hash of command, config file content, and tool version
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Stamp(pub(crate) Xxhash);

#[derive(Clone, Debug)]
pub(crate) struct Tool {
    pub(crate) name: Option<String>,
    pub(crate) cmd: String,
    pub(crate) files: GlobSet,
    pub(crate) ignore: Option<GlobSet>,
    pub(crate) args: Args,
    pub(crate) include_unchanged: bool,
    pub(crate) stamp: Stamp,
    pub(crate) cd: Option<PathBuf>,
    pub(crate) configs: Vec<PathBuf>,
    /// Whether this tool may modify files in-place (formatters, linters in fix mode).
    /// When true, `done()` re-reads file content/mtime after running to avoid caching stale state.
    pub(crate) modifies_files: bool,
}

impl Tool {
    pub(crate) fn display_name(&self) -> &str {
        self.name.as_ref().unwrap_or(&self.cmd)
    }
}
