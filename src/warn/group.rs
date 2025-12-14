use std::str::FromStr;

use crate::warn::warn::Warn;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Group {
    /// All lints
    All,
    /// Pedantic lints
    Pedantic,
}

impl Group {
    /// Expand a lint group into its constituent lints.
    pub(crate) fn warns(self) -> &'static [Warn] {
        match self {
            Group::All => &[
                Warn::UnknownWarning,
                Warn::UnknownTool,
                Warn::UnlistedConfig,
            ],
            Group::Pedantic => &[
                Warn::UnknownWarning,
                Warn::UnknownTool,
                Warn::UnlistedConfig,
                Warn::Careful,
                Warn::Mtime,
                Warn::Refs,
            ],
        }
    }

    /// Get all available groups.
    pub(crate) fn all() -> &'static [Group] {
        &[Group::All, Group::Pedantic]
    }

    pub(crate) fn into_str(self) -> &'static str {
        match self {
            Group::All => "all",
            Group::Pedantic => "pedantic",
        }
    }
}

impl FromStr for Group {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all" => Ok(Group::All),
            "pedantic" => Ok(Group::Pedantic),
            _ => Err(()),
        }
    }
}
