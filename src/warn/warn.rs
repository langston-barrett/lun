use std::{fmt, str::FromStr};

use crate::warn::level;

/// See [`Lint::help`]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Warn {
    #[allow(clippy::enum_variant_names)]
    UnknownWarning,
    UnknownTool,
    UnlistedConfig,
    Careful,
    Mtime,
    Refs,
}

impl fmt::Display for Warn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Warn {
    /// Get the default lint level for this lint.
    pub(crate) fn default_level(self) -> level::Level {
        match self {
            Warn::UnknownWarning => level::Level::Deny,
            Warn::UnknownTool => level::Level::Warn,
            Warn::UnlistedConfig => level::Level::Allow,
            Warn::Careful => level::Level::Allow,
            Warn::Mtime => level::Level::Allow,
            Warn::Refs => level::Level::Allow,
        }
    }

    /// Get the string name of this lint.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Warn::UnknownWarning => "unknown-lint",
            Warn::UnknownTool => "unknown-tool",
            Warn::UnlistedConfig => "unlisted-config",
            Warn::Careful => "careful",
            Warn::Mtime => "mtime",
            Warn::Refs => "refs",
        }
    }

    /// Get the Markdown documentation for a lint.
    pub(crate) fn doc(self) -> &'static str {
        match self {
            Warn::UnknownWarning => include_str!("../../doc/warns/unknown-warning.md"),
            Warn::UnknownTool => include_str!("../../doc/warns/unknown-tool.md"),
            Warn::UnlistedConfig => include_str!("../../doc/warns/unlisted-config.md"),
            Warn::Careful => include_str!("../../doc/warns/careful.md"),
            Warn::Mtime => include_str!("../../doc/warns/mtime.md"),
            Warn::Refs => include_str!("../../doc/warns/refs.md"),
        }
    }

    /// Get the one-line help string for this lint.
    pub(crate) fn help(self) -> &'static str {
        match self {
            Warn::UnknownWarning => "Unknown warning name",
            Warn::UnknownTool => "Unknown tool name passed to `--skip-tool` or `--only-tool`",
            Warn::UnlistedConfig => "Tool config files that exist but are not in `lun.toml`",
            Warn::Careful => "`careful` is not set at CLI or config level",
            Warn::Mtime => "`mtime` is set on CLI or config file",
            Warn::Refs => "`refs` is used on CLI or config file",
        }
    }

    /// Get all available lints.
    pub(crate) fn all() -> &'static [Warn] {
        &[
            Warn::UnknownWarning,
            Warn::UnknownTool,
            Warn::UnlistedConfig,
            Warn::Careful,
            Warn::Mtime,
            Warn::Refs,
        ]
    }
}

impl FromStr for Warn {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unknown-lint" => Ok(Warn::UnknownWarning),
            "unknown-tool" => Ok(Warn::UnknownTool),
            "unlisted-config" => Ok(Warn::UnlistedConfig),
            "careful" => Ok(Warn::Careful),
            "mtime" => Ok(Warn::Mtime),
            "refs" => Ok(Warn::Refs),
            _ => Err(()),
        }
    }
}
