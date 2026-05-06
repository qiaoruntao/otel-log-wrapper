//! One-line OpenTelemetry setup for services that use `tracing`.
//!
//! ```no_run
//! otel_log_wrapper::init!()?;
//! tracing::info!("service started");
//! # Ok::<(), otel_log_wrapper::Error>(())
//! ```

mod config;
mod error;
mod init;

pub use config::{BuildMetadata, LoggerConfig, LoggerConfigBuilder, LoggerFormat};
pub use error::Error;
pub use init::{init_logger, init_logger_guard, LoggerGuard, DEFAULT_ENDPOINT, DEFAULT_PROTOCOL};
pub use init_tracing_opentelemetry;
pub use opentelemetry;
pub use tracing;

#[macro_export]
macro_rules! init {
    () => {
        $crate::LoggerConfig::builder(env!("CARGO_PKG_NAME"))
            .service_version(env!("CARGO_PKG_VERSION"))
            .init()
    };
}

#[doc(hidden)]
pub mod __private {
    pub use crate::init::{apply_env_defaults, effective_log_directives};
}
