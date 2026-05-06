use otel_log_wrapper::__private::{apply_env_defaults, effective_log_directives};
use otel_log_wrapper::{LoggerConfig, LoggerFormat};
use serial_test::serial;
use tracing::level_filters::LevelFilter;

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
        .log_directives("my_bin=info,hyper=off")
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
    assert_eq!(
        config.log_directives.as_deref(),
        Some("my_bin=info,hyper=off")
    );
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
    let attrs = std::env::var("OTEL_RESOURCE_ATTRIBUTES").unwrap();
    assert!(attrs.contains("vcs.commit.short=abc1234"));
    #[cfg(feature = "detect-host")]
    assert!(attrs.contains("host.name="));

    unsafe {
        std::env::remove_var("OTEL_SERVICE_NAME");
        std::env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
        std::env::remove_var("OTEL_EXPORTER_OTLP_PROTOCOL");
        std::env::remove_var("OTEL_RESOURCE_ATTRIBUTES");
    }
}

#[test]
#[serial]
fn effective_log_directives_uses_local_when_env_absent() {
    unsafe {
        std::env::remove_var("RUST_LOG");
    }

    let config = LoggerConfig::builder("test-service")
        .log_directives("my_bin=info,hyper=off")
        .build();

    assert_eq!(
        effective_log_directives(&config).as_deref(),
        Some("my_bin=info,hyper=off")
    );
}

#[test]
#[serial]
fn effective_log_directives_prefers_rust_log_on_conflict() {
    unsafe {
        std::env::set_var("RUST_LOG", "my_bin=debug,hyper=info");
    }

    let config = LoggerConfig::builder("test-service")
        .log_directives("my_bin=info,hyper=off")
        .build();

    assert_eq!(
        effective_log_directives(&config).as_deref(),
        Some("my_bin=info,hyper=off,my_bin=debug,hyper=info")
    );

    unsafe {
        std::env::remove_var("RUST_LOG");
    }
}

#[test]
#[serial]
fn effective_log_directives_uses_rust_log_without_local_directives() {
    unsafe {
        std::env::set_var("RUST_LOG", "warn,my_bin=debug");
    }

    let config = LoggerConfig::builder("test-service").build();

    assert_eq!(
        effective_log_directives(&config).as_deref(),
        Some("warn,my_bin=debug")
    );

    unsafe {
        std::env::remove_var("RUST_LOG");
    }
}
