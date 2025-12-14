use std::collections::HashSet;
use std::str::FromStr as _;

use anyhow::bail;
use tracing::{error, warn};

use crate::cli::warn::WarnOpts;
use crate::warn::group;
use crate::warn::{level, warn::Warn};

#[derive(Debug, Default)]
pub(crate) struct Warns {
    pub(crate) allow: HashSet<Warn>,
    pub(crate) warn: HashSet<Warn>,
    pub(crate) deny: HashSet<Warn>,
}

impl Warns {
    fn allow(&mut self, warn: Warn) {
        self.allow.insert(warn);
        self.warn.remove(&warn);
        self.deny.remove(&warn);
    }

    fn warn(&mut self, warn: Warn) {
        self.warn.insert(warn);
        self.allow.remove(&warn);
        self.deny.remove(&warn);
    }

    fn deny(&mut self, warn: Warn) {
        self.deny.insert(warn);
        self.allow.remove(&warn);
        self.warn.remove(&warn);
    }

    pub(crate) fn level(&self, warn: Warn) -> level::Level {
        if self.allow.contains(&warn) {
            level::Level::Allow
        } else if self.warn.contains(&warn) {
            level::Level::Warn
        } else if self.deny.contains(&warn) {
            level::Level::Deny
        } else {
            warn.default_level()
        }
    }

    /// Returns a list of unknown warnings and the level they were specified at.
    fn process_warnings(
        &mut self,
        allow: &[String],
        warn: &[String],
        deny: &[String],
    ) -> Vec<(level::Level, String)> {
        let mut unknown_wanrs = Vec::new();

        for name in allow {
            if let Ok(group) = group::Group::from_str(name) {
                for &warn in group.warns() {
                    self.allow(warn);
                }
            } else if let Ok(l) = Warn::from_str(name) {
                self.allow(l);
            } else {
                unknown_wanrs.push((level::Level::Allow, name.clone()));
            }
        }
        for name in warn {
            if let Ok(group) = group::Group::from_str(name) {
                for &warn in group.warns() {
                    self.warn(warn);
                }
            } else if let Ok(l) = Warn::from_str(name) {
                self.warn(l);
            } else {
                unknown_wanrs.push((level::Level::Warn, name.clone()));
            }
        }
        for name in deny {
            if let Ok(group) = group::Group::from_str(name) {
                for &warn in group.warns() {
                    self.deny(warn);
                }
            } else if let Ok(l) = Warn::from_str(name) {
                self.deny(l);
            } else {
                unknown_wanrs.push((level::Level::Deny, name.clone()));
            }
        }

        unknown_wanrs
    }
}

impl Warns {
    pub(crate) fn from_cli_and_config(
        cli_opts: &WarnOpts,
        config: Option<&crate::config::Config>,
    ) -> Result<Self, anyhow::Error> {
        let mut warns = Warns::default();
        let mut cli_unknown_warns = Vec::new();
        let mut config_unknown_warns = Vec::new();

        if let Some(config) = config {
            config_unknown_warns.extend(warns.process_warnings(
                &config.warns.allow,
                &config.warns.warn,
                &config.warns.deny,
            ));
        }
        cli_unknown_warns.extend(warns.process_warnings(
            &cli_opts.allow,
            &cli_opts.warn,
            &cli_opts.deny,
        ));

        let unknown_warn_level = warns.level(Warn::UnknownWarning);
        match unknown_warn_level {
            level::Level::Allow => {}
            level::Level::Warn => {
                for (level, name) in &cli_unknown_warns {
                    warn!("unknown warning `{name}` specified in `--{level}`");
                }
                for (level, name) in &config_unknown_warns {
                    warn!("unknown warning `{name}` specified in `{level}`");
                }
            }
            level::Level::Deny => {
                for (level, name) in &cli_unknown_warns {
                    error!("unknown warning `{name}` specified in `--{level}`");
                }
                for (level, name) in &cli_unknown_warns {
                    error!("unknown warning `{name}` specified in `--{level}`");
                }
                if !cli_unknown_warns.is_empty() {
                    bail!(
                        "found unknown warning names and --deny={}",
                        Warn::UnknownWarning.as_str()
                    );
                }
            }
        }

        Ok(warns)
    }
}

impl TryFrom<&WarnOpts> for Warns {
    type Error = anyhow::Error;

    fn try_from(opts: &WarnOpts) -> Result<Self, Self::Error> {
        Self::from_cli_and_config(opts, None)
    }
}
