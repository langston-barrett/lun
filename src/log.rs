use std::io::IsTerminal;

use tracing::{Level, level_filters::LevelFilter};
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

use crate::cli::log;

fn verbosity_to_log_level(verbosity: u8) -> Level {
    match verbosity {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    }
}

#[allow(clippy::unwrap_used)]
pub(crate) fn init_tracing(opts: log::LogOptions) {
    let effective_verbosity = opts.verbose.saturating_sub(opts.quiet);
    let verbose = verbosity_to_log_level(effective_verbosity + 1);
    let ansi = match opts.color {
        log::Color::Always => true,
        log::Color::Never => false,
        log::Color::Auto => std::io::stdout().is_terminal(),
    };
    let tgts = tracing_subscriber::filter::Targets::new()
        .with_target(env!("CARGO_CRATE_NAME"), verbose)
        .with_target("regex", LevelFilter::OFF)
        .with_default(LevelFilter::OFF);
    if opts.log_timestamp {
        let builder = tracing_subscriber::fmt::fmt()
            .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
            .with_max_level(verbose)
            .with_target(false)
            .with_ansi(ansi);
        builder.finish().with(tgts).try_init().unwrap();
    } else {
        let builder = tracing_subscriber::fmt::fmt()
            .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
            .with_max_level(verbose)
            .with_target(false)
            .with_ansi(ansi)
            .without_time();
        builder.finish().with(tgts).try_init().unwrap();
    }
}
