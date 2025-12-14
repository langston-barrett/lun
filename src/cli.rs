use std::{num::NonZeroUsize, path::PathBuf};

pub(crate) mod log;
pub(crate) mod warn;

#[derive(Debug, clap::Parser)]
#[command(name = "lun")]
#[command(about = "Run linters fast")]
#[command(version)]
pub(crate) struct Cli {
    /// Path to the cache directory
    #[arg(long, default_value = ".lun")]
    pub(crate) cache: PathBuf,
    /// Path to the configuration file
    #[arg(short, long, default_value = "lun.toml")]
    pub(crate) config: PathBuf,
    #[command(flatten)]
    pub(crate) log: log::LogOptions,
    #[command(flatten)]
    pub(crate) warn: warn::WarnOpts,
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Run(Run),
    /// Delete the cache
    Clean,
    Init(Init),
    Add(Add),
    /// Show available warnings
    Warns {
        /// Show documentation for a specific warnings
        #[arg(value_name = "WARN")]
        warn: Option<String>,
    },
}

/// Run linters and formatters
#[derive(Debug, clap::Parser)]
pub(crate) struct Run {
    /// Include tool version in cache keys
    #[arg(long)]
    pub(crate) careful: bool,
    /// Run linters, run formatters in "check" mode (i.e., in CI)
    #[arg(long)]
    pub(crate) check: bool,
    /// Don't execute any commands
    #[arg(short = 'n', long)]
    pub(crate) dry_run: bool,
    /// Command to run on failure (see also --then)
    #[arg(short, long)]
    pub(crate) r#else: Option<String>,
    /// Run tools in fix mode (that have them)
    #[arg(short = 'x', long)]
    pub(crate) fix: bool,
    /// Only run formatters
    #[arg(short, long = "format")]
    pub(crate) format: bool,
    /// Number of parallel jobs (overrides config file value)
    #[arg(short, long = "jobs")]
    pub(crate) jobs: Option<NonZeroUsize>,
    /// Continue running commands even after one fails
    #[arg(long)]
    pub(crate) keep_going: bool,
    /// Use mtime to skip unchanged files (overrides config file value)
    #[arg(short, long)]
    pub(crate) mtime: bool,
    /// Use Ninja to run commands (overrides config file value)
    #[arg(short = 'N', long)]
    pub(crate) ninja: bool,
    /// Skip batching jobs (run one command per file)
    #[arg(long)]
    pub(crate) no_batch: bool,
    /// Don't capture output (stream directly to terminal)
    #[arg(long)]
    pub(crate) no_capture: bool,
    /// Only run tools with the given name (can be used multiple times)
    #[arg(long, action = clap::ArgAction::Append, value_name = "TOOL")]
    pub(crate) only_tool: Vec<String>,
    /// Skip tools with the given name (can be used multiple times)
    #[arg(long, action = clap::ArgAction::Append, value_name = "TOOL")]
    pub(crate) skip_tool: Vec<String>,
    /// Only run on matching files (can be used multiple times)
    #[arg(long, action = clap::ArgAction::Append, value_name = "GLOB")]
    pub(crate) only_files: Vec<String>,
    /// Skip matching files (can be used multiple times)
    #[arg(long, action = clap::ArgAction::Append, value_name = "GLOB")]
    pub(crate) skip_files: Vec<String>,
    /// Only run on staged files (useful in pre-commit hooks)
    #[arg(long)]
    pub(crate) staged: bool,
    /// Command to run failure (useful with --watch)
    #[arg(short, long)]
    pub(crate) then: Option<String>,
    /// Git refs assumed to be good (can be used multiple times)
    #[arg(long, action = clap::ArgAction::Append)]
    pub(crate) refs: Vec<String>,
    /// Watch for file changes and re-run automatically
    #[arg(long)]
    pub(crate) watch: bool,
}

/// Create a config file with detected linters and formatters
#[derive(Debug, clap::Parser)]
pub(crate) struct Init {
    /// Add a tool (can be used multiple times)
    #[arg(long, action = clap::ArgAction::Append)]
    pub(crate) linter: Vec<String>,
}

/// Add a tool to the config file
#[derive(Debug, clap::Parser)]
pub(crate) struct Add {
    /// Name of the tool to add
    pub(crate) tool: String,
    /// Override the tool name
    #[arg(long)]
    pub(crate) name: Option<String>,
    /// Override whether this is a formatter
    #[arg(long)]
    pub(crate) formatter: Option<bool>,
    /// Override the command
    #[arg(long)]
    pub(crate) cmd: Option<String>,
    /// Override the file glob pattern
    #[arg(long)]
    pub(crate) files: Option<String>,
    /// Override the check command
    #[arg(long)]
    pub(crate) check: Option<String>,
    /// Override the config file path
    #[arg(long)]
    pub(crate) config: Option<PathBuf>,
}
