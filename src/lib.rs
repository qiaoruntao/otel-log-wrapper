//! One-line OpenTelemetry setup for services that use `tracing`.
//!
//! ```no_run
//! otel_log_wrapper::init!()?;
//! tracing::info!("service started");
//! # Ok::<(), otel_log_wrapper::Error>(())
//! ```

use std::fmt;
use std::sync::OnceLock;

use init_tracing_opentelemetry::resource::DetectResource;
use init_tracing_opentelemetry::{LogFormat, TracingConfig};
use tracing::level_filters::LevelFilter;

pub use init_tracing_opentelemetry;
pub use opentelemetry;
pub use tracing;

pub const DEFAULT_ENDPOINT: &str = "http://localhost:4317";
pub const DEFAULT_PROTOCOL: &str = "grpc";

static GLOBAL_GUARD: OnceLock<LoggerGuard> = OnceLock::new();

pub type LoggerGuard = init_tracing_opentelemetry::Guard;

#[macro_export]
macro_rules! init {
    () => {
        $crate::LoggerConfig::builder(env!("CARGO_PKG_NAME"))
            .service_version(env!("CARGO_PKG_VERSION"))
            .init()
    };
}

#[derive(Debug)]
pub enum Error {
    Init(init_tracing_opentelemetry::Error),
    AlreadyInitialized,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Init(err) => write!(f, "{err}"),
            Self::AlreadyInitialized => write!(f, "logger already initialized"),
        }
    }
}

impl std::error::Error for Error {}

impl From<init_tracing_opentelemetry::Error> for Error {
    fn from(value: init_tracing_opentelemetry::Error) -> Self {
        Self::Init(value)
    }
}

#[derive(Debug, Clone)]
pub struct LoggerConfig {
    pub service_name: &'static str,
    pub service_version: Option<&'static str>,
    pub endpoint: Option<String>,
    pub protocol: Option<String>,
    pub log_directives: Option<String>,
    pub default_level: LevelFilter,
    pub format: LoggerFormat,
    pub metrics: bool,
    pub global_subscriber: bool,
    pub startup_message: bool,
    pub build_metadata: BuildMetadata,
}

impl LoggerConfig {
    #[must_use]
    pub fn builder(service_name: &'static str) -> LoggerConfigBuilder {
        LoggerConfigBuilder {
            inner: Self::new(service_name),
        }
    }

    #[must_use]
    pub fn new(service_name: &'static str) -> Self {
        Self {
            service_name,
            service_version: None,
            endpoint: None,
            protocol: None,
            log_directives: None,
            default_level: LevelFilter::INFO,
            format: LoggerFormat::default(),
            metrics: true,
            global_subscriber: true,
            startup_message: true,
            build_metadata: BuildMetadata::default(),
        }
    }

    pub fn init(self) -> Result<&'static LoggerGuard, Error> {
        init_logger(self)
    }

    pub fn init_guard(self) -> Result<LoggerGuard, Error> {
        init_logger_guard(self)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum LoggerFormat {
    #[default]
    Human,
    Pretty,
    Full,
    Compact,
    Json,
}

impl LoggerFormat {
    fn apply(self, config: TracingConfig) -> TracingConfig {
        match self {
            Self::Human | Self::Full => config.with_format(LogFormat::Full),
            Self::Pretty => config.with_format(LogFormat::Pretty),
            Self::Compact => config.with_format(LogFormat::Compact),
            Self::Json => config.with_format(LogFormat::Json),
        }
    }
}

impl fmt::Display for LoggerFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Human => f.write_str("human"),
            Self::Pretty => f.write_str("pretty"),
            Self::Full => f.write_str("full"),
            Self::Compact => f.write_str("compact"),
            Self::Json => f.write_str("json"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BuildMetadata {
    pub commit_short: Option<&'static str>,
    pub commit_hash: Option<&'static str>,
    pub branch: Option<&'static str>,
    pub build_time: Option<&'static str>,
}

impl BuildMetadata {
    fn resource_attributes(self) -> impl Iterator<Item = (&'static str, &'static str)> {
        [
            ("vcs.commit.short", self.commit_short),
            ("vcs.commit.hash", self.commit_hash),
            ("vcs.branch", self.branch),
            ("build.time", self.build_time),
        ]
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| (key, value)))
    }
}

#[derive(Debug)]
pub struct LoggerConfigBuilder {
    inner: LoggerConfig,
}

impl LoggerConfigBuilder {
    #[must_use]
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.inner.endpoint = Some(endpoint.into());
        self
    }

    #[must_use]
    pub fn protocol(mut self, protocol: impl Into<String>) -> Self {
        self.inner.protocol = Some(protocol.into());
        self
    }

    #[must_use]
    pub fn log_directives(mut self, directives: impl Into<String>) -> Self {
        self.inner.log_directives = Some(directives.into());
        self
    }

    #[must_use]
    pub fn default_level(mut self, level: LevelFilter) -> Self {
        self.inner.default_level = level;
        self
    }

    #[must_use]
    pub fn format(mut self, format: LoggerFormat) -> Self {
        self.inner.format = format;
        self
    }

    #[must_use]
    pub fn metrics(mut self, enabled: bool) -> Self {
        self.inner.metrics = enabled;
        self
    }

    #[must_use]
    pub fn global_subscriber(mut self, enabled: bool) -> Self {
        self.inner.global_subscriber = enabled;
        self
    }

    #[must_use]
    pub fn startup_message(mut self, enabled: bool) -> Self {
        self.inner.startup_message = enabled;
        self
    }

    #[must_use]
    pub fn service_version(mut self, version: &'static str) -> Self {
        self.inner.service_version = Some(version);
        self
    }

    #[must_use]
    pub fn commit_short(mut self, commit: &'static str) -> Self {
        self.inner.build_metadata.commit_short = non_empty(commit);
        self
    }

    #[must_use]
    pub fn commit_hash(mut self, commit: &'static str) -> Self {
        self.inner.build_metadata.commit_hash = non_empty(commit);
        self
    }

    #[must_use]
    pub fn branch(mut self, branch: &'static str) -> Self {
        self.inner.build_metadata.branch = non_empty(branch);
        self
    }

    #[must_use]
    pub fn build_time(mut self, build_time: &'static str) -> Self {
        self.inner.build_metadata.build_time = non_empty(build_time);
        self
    }

    #[must_use]
    pub fn build_metadata(mut self, metadata: BuildMetadata) -> Self {
        self.inner.build_metadata = metadata;
        self
    }

    #[must_use]
    pub fn build(self) -> LoggerConfig {
        self.inner
    }

    pub fn init(self) -> Result<&'static LoggerGuard, Error> {
        self.build().init()
    }

    pub fn init_guard(self) -> Result<LoggerGuard, Error> {
        self.build().init_guard()
    }
}

pub fn init_logger(config: LoggerConfig) -> Result<&'static LoggerGuard, Error> {
    if let Some(guard) = GLOBAL_GUARD.get() {
        return Ok(guard);
    }

    let guard = init_logger_guard(config)?;
    GLOBAL_GUARD
        .set(guard)
        .map_err(|_| Error::AlreadyInitialized)?;
    Ok(GLOBAL_GUARD
        .get()
        .expect("global logger guard was just initialized"))
}

pub fn init_logger_guard(config: LoggerConfig) -> Result<LoggerGuard, Error> {
    apply_env_defaults(&config);

    if config.startup_message {
        print_startup_message(&config);
    }

    let mut resource = DetectResource::default().with_fallback_service_name(config.service_name);
    if let Some(version) = config.service_version {
        resource = resource.with_fallback_service_version(version);
    }

    let tracing_config = config
        .format
        .apply(TracingConfig::production())
        .with_default_level(config.default_level)
        .with_metrics(config.metrics)
        .with_global_subscriber(config.global_subscriber)
        .with_otel_tracer_name(config.service_name)
        .with_resource_config(resource);

    let tracing_config = if let Some(directives) = config.log_directives {
        tracing_config.with_log_directives(directives)
    } else {
        tracing_config
    };

    tracing_config.init_subscriber().map_err(Error::from)
}

fn apply_env_defaults(config: &LoggerConfig) {
    set_env_if_missing("OTEL_SERVICE_NAME", config.service_name);
    set_env_if_missing(
        "OTEL_EXPORTER_OTLP_ENDPOINT",
        config.endpoint.as_deref().unwrap_or(DEFAULT_ENDPOINT),
    );
    set_env_if_missing(
        "OTEL_EXPORTER_OTLP_PROTOCOL",
        config.protocol.as_deref().unwrap_or(DEFAULT_PROTOCOL),
    );
    merge_resource_attributes(config.build_metadata.resource_attributes());
}

fn set_env_if_missing(key: &str, value: &str) {
    if std::env::var_os(key).is_some() {
        return;
    }

    // OpenTelemetry setup must happen at process startup before worker threads
    // are spawned. This crate only writes env vars when no explicit value exists.
    unsafe {
        std::env::set_var(key, value);
    }
}

fn merge_resource_attributes(attributes: impl Iterator<Item = (&'static str, &'static str)>) {
    let mut attributes: Vec<String> = attributes
        .map(|(key, value)| format!("{key}={value}"))
        .collect();
    if attributes.is_empty() {
        return;
    }

    if let Ok(existing) = std::env::var("OTEL_RESOURCE_ATTRIBUTES") {
        if !existing.trim().is_empty() {
            attributes.insert(0, existing);
        }
    }

    unsafe {
        std::env::set_var("OTEL_RESOURCE_ATTRIBUTES", attributes.join(","));
    }
}

fn print_startup_message(config: &LoggerConfig) {
    let endpoint = config.endpoint.as_deref().unwrap_or(DEFAULT_ENDPOINT);
    let protocol = config.protocol.as_deref().unwrap_or(DEFAULT_PROTOCOL);
    let version = config.service_version.unwrap_or("unknown");
    let commit = config.build_metadata.commit_short.unwrap_or("unknown");
    let level = config
        .log_directives
        .as_deref()
        .unwrap_or("RUST_LOG|OTEL_LOG_LEVEL|info");

    println!(
        "otel-log-wrapper init service={} version={} commit={} endpoint={} protocol={} format={} metrics={} level={}",
        config.service_name,
        version,
        commit,
        endpoint,
        protocol,
        config.format,
        config.metrics,
        level
    );
}

fn non_empty(value: &'static str) -> Option<&'static str> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    fn default_config_matches_zero_config_expectations() {
        let config = LoggerConfig::new("test-service");

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.endpoint, None);
        assert_eq!(config.protocol, None);
        assert_eq!(config.default_level, LevelFilter::INFO);
        assert!(matches!(config.format, LoggerFormat::Human));
        assert!(config.metrics);
        assert!(config.global_subscriber);
        assert!(config.startup_message);
    }

    #[test]
    fn builder_sets_overrides() {
        let config = LoggerConfig::builder("test-service")
            .endpoint("http://collector:4317")
            .protocol("grpc")
            .log_directives("debug")
            .default_level(LevelFilter::DEBUG)
            .format(LoggerFormat::Compact)
            .metrics(false)
            .global_subscriber(false)
            .startup_message(false)
            .service_version("1.2.3")
            .commit_short("abc1234")
            .commit_hash("abc123456789")
            .branch("main")
            .build_time("2026-04-26T00:00:00Z")
            .build();

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.endpoint.as_deref(), Some("http://collector:4317"));
        assert_eq!(config.protocol.as_deref(), Some("grpc"));
        assert_eq!(config.log_directives.as_deref(), Some("debug"));
        assert_eq!(config.default_level, LevelFilter::DEBUG);
        assert!(matches!(config.format, LoggerFormat::Compact));
        assert!(!config.metrics);
        assert!(!config.global_subscriber);
        assert!(!config.startup_message);
        assert_eq!(config.service_version, Some("1.2.3"));
        assert_eq!(config.build_metadata.commit_short, Some("abc1234"));
        assert_eq!(config.build_metadata.commit_hash, Some("abc123456789"));
        assert_eq!(config.build_metadata.branch, Some("main"));
        assert_eq!(
            config.build_metadata.build_time,
            Some("2026-04-26T00:00:00Z")
        );
    }

    #[test]
    #[serial]
    fn env_defaults_do_not_override_existing_values() {
        unsafe {
            std::env::set_var("OTEL_SERVICE_NAME", "existing-service");
            std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://existing:4317");
            std::env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf");
            std::env::remove_var("OTEL_RESOURCE_ATTRIBUTES");
        }

        apply_env_defaults(
            &LoggerConfig::builder("new-service")
                .endpoint("http://new:4317")
                .protocol("grpc")
                .commit_short("abc1234")
                .build(),
        );

        assert_eq!(
            std::env::var("OTEL_SERVICE_NAME").as_deref(),
            Ok("existing-service")
        );
        assert_eq!(
            std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").as_deref(),
            Ok("http://existing:4317")
        );
        assert_eq!(
            std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL").as_deref(),
            Ok("http/protobuf")
        );
        assert_eq!(
            std::env::var("OTEL_RESOURCE_ATTRIBUTES").as_deref(),
            Ok("vcs.commit.short=abc1234")
        );

        unsafe {
            std::env::remove_var("OTEL_SERVICE_NAME");
            std::env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
            std::env::remove_var("OTEL_EXPORTER_OTLP_PROTOCOL");
            std::env::remove_var("OTEL_RESOURCE_ATTRIBUTES");
        }
    }
}
