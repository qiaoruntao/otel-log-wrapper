use std::sync::OnceLock;

use init_tracing_opentelemetry::resource::DetectResource;
use init_tracing_opentelemetry::TracingConfig;

use crate::{Error, LoggerConfig};

pub const DEFAULT_ENDPOINT: &str = "http://localhost:4317";
pub const DEFAULT_PROTOCOL: &str = "grpc";

static GLOBAL_GUARD: OnceLock<LoggerGuard> = OnceLock::new();

pub type LoggerGuard = init_tracing_opentelemetry::Guard;

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

    let effective_log_directives = effective_log_directives(&config);

    if config.startup_message {
        print_startup_message(&config, effective_log_directives.as_deref());
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

    let tracing_config = if let Some(directives) = effective_log_directives {
        tracing_config.with_log_directives(directives)
    } else {
        tracing_config
    };

    tracing_config.init_subscriber().map_err(Error::from)
}

pub fn apply_env_defaults(config: &LoggerConfig) {
    set_env_if_missing("OTEL_SERVICE_NAME", config.service_name);
    set_env_if_missing(
        "OTEL_EXPORTER_OTLP_ENDPOINT",
        config.endpoint.as_deref().unwrap_or(DEFAULT_ENDPOINT),
    );
    set_env_if_missing(
        "OTEL_EXPORTER_OTLP_PROTOCOL",
        config.protocol.as_deref().unwrap_or(DEFAULT_PROTOCOL),
    );
    merge_resource_attributes(
        config
            .build_metadata
            .resource_attributes()
            .chain(detect_host_resource_attribute()),
    );
}

pub fn effective_log_directives(config: &LoggerConfig) -> Option<String> {
    let local = config
        .log_directives
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let rust_log = std::env::var("RUST_LOG").ok().and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    match (local, rust_log) {
        (Some(local), Some(rust_log)) => Some(format!("{local},{rust_log}")),
        (Some(local), None) => Some(local.to_string()),
        (None, Some(rust_log)) => Some(rust_log),
        (None, None) => None,
    }
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

#[cfg(feature = "detect-host")]
fn detect_host_resource_attribute() -> impl Iterator<Item = (&'static str, &'static str)> {
    hostname::get()
        .ok()
        .and_then(|value| value.into_string().ok())
        .filter(|value| !value.trim().is_empty())
        .map(leak_owned_string)
        .map(|host| ("host.name", host))
        .into_iter()
}

#[cfg(not(feature = "detect-host"))]
fn detect_host_resource_attribute() -> impl Iterator<Item = (&'static str, &'static str)> {
    std::iter::empty()
}

fn leak_owned_string(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn print_startup_message(config: &LoggerConfig, effective_log_directives: Option<&str>) {
    let endpoint = config.endpoint.as_deref().unwrap_or(DEFAULT_ENDPOINT);
    let protocol = config.protocol.as_deref().unwrap_or(DEFAULT_PROTOCOL);
    let version = config.service_version.unwrap_or("unknown");
    let commit = config.build_metadata.commit_short.unwrap_or("unknown");
    let level = effective_log_directives.unwrap_or("RUST_LOG|OTEL_LOG_LEVEL|info");

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
