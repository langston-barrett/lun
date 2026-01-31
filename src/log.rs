use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

use crate::out;

#[allow(clippy::unwrap_used)]
pub(crate) fn init_tracing(out_config: out::Config) {
    let tgts = tracing_subscriber::filter::Targets::new()
        .with_target(env!("CARGO_CRATE_NAME"), out_config.verbosity)
        .with_target("regex", LevelFilter::OFF)
        .with_default(LevelFilter::OFF);
    if out_config.timestamps {
        let builder = tracing_subscriber::fmt::fmt()
            .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
            .with_max_level(out_config.verbosity)
            .with_target(false)
            .with_ansi(out_config.color);
        builder.finish().with(tgts).try_init().unwrap();
    } else {
        let builder = tracing_subscriber::fmt::fmt()
            .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
            .with_max_level(out_config.verbosity)
            .with_target(false)
            .with_ansi(out_config.color)
            .without_time();
        builder.finish().with(tgts).try_init().unwrap();
    }
}
