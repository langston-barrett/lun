#[derive(Clone, Debug, clap::Args)]
pub(crate) struct WarnOpts {
    /// Allow a warning (can be used multiple times)
    #[arg(short = 'A', long, action = clap::ArgAction::Append, value_name = "WARN", help_heading = "Warning options")]
    pub(crate) allow: Vec<String>,
    /// Warn for a warning (can be used multiple times)
    #[arg(short = 'W', long, action = clap::ArgAction::Append, value_name = "WARN", help_heading = "Warning options")]
    pub(crate) warn: Vec<String>,
    /// Deny a warning (can be used multiple times)
    #[arg(short = 'D', long, action = clap::ArgAction::Append, value_name = "WARN", help_heading = "Warning options")]
    pub(crate) deny: Vec<String>,
}
