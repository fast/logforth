use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use log::Record;
use opentelemetry::logs::{AnyValue, LoggerProvider as ILoggerProvider};
use opentelemetry::logs::{Logger, Severity};
use opentelemetry::InstrumentationLibrary;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::logs::LoggerProvider;

use crate::append::Append;

#[derive(Debug)]
pub struct OpenTelemetryLog {
    name: String,
    category: String,
    library: Arc<InstrumentationLibrary>,
    provider: LoggerProvider,
}

impl OpenTelemetryLog {
    pub fn new(
        name: impl Into<String>,
        category: impl Into<String>,
        otlp_endpoint: impl Into<String>,
    ) -> Self {
        let name = name.into();
        let category = category.into();
        let otlp_endpoint = otlp_endpoint.into();

        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(otlp_endpoint)
            .with_protocol(opentelemetry_otlp::Protocol::Grpc)
            .with_timeout(Duration::from_secs(
                opentelemetry_otlp::OTEL_EXPORTER_OTLP_TIMEOUT_DEFAULT,
            ))
            .build_log_exporter()
            .expect("failed to initialize oltp exporter");

        let provider = LoggerProvider::builder()
            .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
            .build();

        let library = Arc::new(InstrumentationLibrary::builder(name.clone()).build());

        Self {
            name,
            category,
            library,
            provider,
        }
    }
}

impl Append for OpenTelemetryLog {
    fn try_append(&self, log_record: &Record) -> anyhow::Result<()> {
        let provider = self.provider.clone();
        let logger = provider.library_logger(self.library.clone());

        let mut record = opentelemetry_sdk::logs::LogRecord::default();
        record.observed_timestamp = Some(SystemTime::now());
        record.severity_number = Some(log_level_to_otel_severity(log_record.level()));
        record.severity_text = Some(log_record.level().as_str().into());
        record.body = Some(AnyValue::from(log_record.args().to_string()));

        logger.emit(record);
        Ok(())
    }

    fn flush(&self) {
        for err in self
            .provider
            .force_flush()
            .into_iter()
            .filter_map(|r| r.err())
        {
            eprintln!(
                "failed to flush logger ({}@{}): {}",
                self.name, self.category, err
            );
        }
    }
}

fn log_level_to_otel_severity(level: log::Level) -> Severity {
    match level {
        log::Level::Error => Severity::Error,
        log::Level::Warn => Severity::Warn,
        log::Level::Info => Severity::Info,
        log::Level::Debug => Severity::Debug,
        log::Level::Trace => Severity::Trace,
    }
}
