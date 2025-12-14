#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub(crate) enum Color {
    /// Always use color output
    Always,
    /// Never use color output
    Never,
    /// Use color output if stdout is a terminal
    #[default]
    Auto,
}

#[derive(Clone, Copy, Debug, clap::Args)]
#[group(id = "log")]
pub(crate) struct LogOptions {
    /// When to use color output
    #[arg(long, default_value = "auto", help_heading = "Logging options")]
    pub(crate) color: Color,
    /// Include timestamps in log output
    #[arg(long, help_heading = "Logging options")]
    pub(crate) log_timestamp: bool,
    /// Quiet mode (can be used multiple times, opposite of `--verbose`)
    #[arg(short, long, action = clap::ArgAction::Count, help_heading = "Logging options")]
    pub(crate) quiet: u8,
    /// Verbosity level (can be used multiple times)
    #[arg(short, long, action = clap::ArgAction::Count, help_heading = "Logging options")]
    pub(crate) verbose: u8,
}
