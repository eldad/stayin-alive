use tracing_subscriber::{EnvFilter, fmt, fmt::format::FmtSpan, prelude::*};

pub fn setup_logging() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "stayin_alive=info,tower_http=info".parse().unwrap());

    let fmt_layer = fmt::layer().with_span_events(FmtSpan::CLOSE);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}
