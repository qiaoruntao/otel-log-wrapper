use std::fmt;

use init_tracing_opentelemetry::{LogFormat, TracingConfig};
use tracing::level_filters::LevelFilter;

use crate::{init_logger, init_logger_guard, Error, LoggerGuard};

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

    /// Normalizes a Cargo identifier (e.g. `env!("CARGO_BIN_NAME")` or
    /// `env!("CARGO_PKG_NAME")`) into the form `tracing` actually uses as
    /// an event/span target.
    ///
    /// Rust silently replaces `-` with `_` when turning a crate/binary name
    /// into a module path, so a hyphenated Cargo name (any binary declared
    /// with a hyphen, e.g. `my-service`) never matches a `tracing_subscriber`
    /// filter directive built from the raw hyphenated string: `my-service=info`
    /// filters against the literal target `my-service`, but every event in
    /// that binary is actually emitted under the target `my_service`. The
    /// directive silently falls through to whatever default level applies
    /// (commonly `off`), so no logs, spans, or otel export ever happen --
    /// with no error, warning, or crash to point at the cause.
    ///
    /// Always build filter directives from this normalized form, not the
    /// raw Cargo name:
    ///
    /// ```no_run
    /// use otel_log_wrapper::LoggerConfig;
    ///
    /// // In practice pass env!("CARGO_BIN_NAME") here; hardcoded for the doctest.
    /// let bin_name = "my-service";
    /// let target = LoggerConfig::normalize_target(bin_name);
    /// LoggerConfig::builder(bin_name)
    ///     .log_directives(format!("off,{target}=info"))
    ///     .startup_message(false)
    ///     .init_guard()
    ///     .ok();
    /// ```
    #[must_use]
    pub fn normalize_target(cargo_name: &str) -> String {
        cargo_name.replace('-', "_")
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
    pub(crate) fn apply(self, config: TracingConfig) -> TracingConfig {
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
    pub(crate) fn resource_attributes(self) -> impl Iterator<Item = (&'static str, &'static str)> {
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

fn non_empty(value: &'static str) -> Option<&'static str> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::LoggerConfig;

    #[test]
    fn normalize_target_replaces_hyphens_with_underscores() {
        assert_eq!(
            LoggerConfig::normalize_target("stripchat-looper"),
            "stripchat_looper"
        );
        assert_eq!(
            LoggerConfig::normalize_target("already_underscored"),
            "already_underscored"
        );
        assert_eq!(
            LoggerConfig::normalize_target("multi-part-name"),
            "multi_part_name"
        );
    }
}
