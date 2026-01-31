use std::io::{self, IsTerminal as _};

use crate::cli::{self, log::LogOptions};

fn verbosity_to_log_level(verbosity: u8) -> tracing::Level {
    match verbosity {
        0 => tracing::Level::WARN,
        1 => tracing::Level::INFO,
        2 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Config {
    pub(crate) color: bool,
    pub(crate) interactive: bool,
    pub(crate) timestamps: bool,
    pub(crate) verbosity: tracing::Level,
}

#[cfg(test)]
impl Default for Config {
    fn default() -> Self {
        Self {
            color: false,
            interactive: false,
            timestamps: false,
            verbosity: tracing::Level::WARN,
        }
    }
}

impl Config {
    pub(crate) fn new(log_opts: LogOptions) -> Self {
        let interactive = if cfg!(test) {
            false
        } else {
            io::stderr().is_terminal()
        };
        let color = match log_opts.color {
            cli::log::Color::Always => true,
            cli::log::Color::Never => false,
            cli::log::Color::Auto => interactive,
        };
        let effective_verbosity = log_opts.verbose.saturating_sub(log_opts.quiet);
        Self {
            color,
            interactive,
            timestamps: log_opts.log_timestamp,
            verbosity: verbosity_to_log_level(effective_verbosity + 1),
        }
    }

    pub(crate) fn color_as_str(&self) -> &'static str {
        if self.color { "always" } else { "never" }
    }
}
