use tracing::info;
use tracing::subscriber::set_global_default;
use tracing_log::LogTracer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;

pub fn setup_logger() {
    LogTracer::init().expect("Failed to set logger");
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = Registry::default()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json());
    set_global_default(subscriber).expect("Failed to set subscriber");

    info!("Logger initialized");
}
