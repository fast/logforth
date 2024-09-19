// Copyright 2024 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use log::Record;
use opentelemetry::logs::AnyValue;
use opentelemetry::logs::LogRecord as _;
use opentelemetry::logs::Logger;
use opentelemetry::logs::LoggerProvider as ILoggerProvider;
use opentelemetry::logs::Severity;
use opentelemetry::InstrumentationLibrary;
use opentelemetry_otlp::Protocol;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::logs::LogRecord;
use opentelemetry_sdk::logs::LoggerProvider;

use crate::append::Append;

/// The communication protocol to opentelemetry that used when exporting data.
///
/// This is a logical re-exported [`Protocol`] to avoid version lock-in to
/// `opentelemetry_otlp`.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpentelemetryWireProtocol {
    /// GRPC protocol
    Grpc,
    /// HTTP protocol with binary protobuf
    HttpBinary,
    /// HTTP protocol with JSON payload
    HttpJson,
}

/// A builder to configure and create an [`OpentelemetryLog`] appender.
pub struct OpentelemetryLogBuilder {
    name: String,
    endpoint: String,
    protocol: Protocol,
    labels: Vec<(Cow<'static, str>, Cow<'static, str>)>,
}

impl OpentelemetryLogBuilder {
    /// Create a new builder with the given name and OTLP endpoint.
    pub fn new(name: impl Into<String>, otlp_endpoint: impl Into<String>) -> Self {
        OpentelemetryLogBuilder {
            name: name.into(),
            endpoint: otlp_endpoint.into(),
            protocol: Protocol::Grpc,
            labels: vec![],
        }
    }

    /// Set the protocol to use when exporting data to opentelemetry.
    ///
    /// Default to [`Grpc`].
    ///
    /// [`Grpc`]: OpentelemetryWireProtocol::Grpc
    pub fn with_protocol(mut self, protocol: OpentelemetryWireProtocol) -> Self {
        self.protocol = match protocol {
            OpentelemetryWireProtocol::Grpc => Protocol::Grpc,
            OpentelemetryWireProtocol::HttpBinary => Protocol::HttpBinary,
            OpentelemetryWireProtocol::HttpJson => Protocol::HttpJson,
        };
        self
    }

    /// Add a label to the resource.
    pub fn add_label(
        mut self,
        key: impl Into<Cow<'static, str>>,
        value: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.labels.push((key.into(), value.into()));
        self
    }

    /// Build the [`OpentelemetryLog`] appender.
    pub fn build(self) -> Result<OpentelemetryLog, opentelemetry::logs::LogError> {
        let OpentelemetryLogBuilder {
            name,
            endpoint,
            protocol,
            labels,
        } = self;

        let collector_timeout =
            Duration::from_secs(opentelemetry_otlp::OTEL_EXPORTER_OTLP_TIMEOUT_DEFAULT);
        let exporter = match protocol {
            Protocol::Grpc => opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint)
                .with_protocol(protocol)
                .with_timeout(collector_timeout)
                .build_log_exporter(),
            Protocol::HttpBinary | Protocol::HttpJson => opentelemetry_otlp::new_exporter()
                .http()
                .with_endpoint(endpoint)
                .with_protocol(protocol)
                .with_timeout(collector_timeout)
                .build_log_exporter(),
        }?;

        let provider = LoggerProvider::builder()
            .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
            .with_resource(opentelemetry_sdk::Resource::new(labels.into_iter().map(
                |(key, value)| opentelemetry::KeyValue {
                    key: key.into(),
                    value: value.into(),
                },
            )))
            .build();

        let library = Arc::new(InstrumentationLibrary::builder(name.clone()).build());

        Ok(OpentelemetryLog {
            name,
            library,
            provider,
        })
    }
}

/// An appender that sends log records to opentelemetry.
#[derive(Debug)]
pub struct OpentelemetryLog {
    name: String,
    library: Arc<InstrumentationLibrary>,
    provider: LoggerProvider,
}

impl Append for OpentelemetryLog {
    fn append(&self, log_record: &Record) -> anyhow::Result<()> {
        let provider = self.provider.clone();
        let logger = provider.library_logger(self.library.clone());

        let mut record = LogRecord::default();
        record.observed_timestamp = Some(SystemTime::now());
        record.severity_number = Some(log_level_to_otel_severity(log_record.level()));
        record.severity_text = Some(log_record.level().as_str());
        record.target = Some(log_record.target().to_string().into());
        record.body = Some(AnyValue::from(log_record.args().to_string()));

        if let Some(module_path) = log_record.module_path() {
            record.add_attribute("module_path", module_path.to_string());
        }
        if let Some(file) = log_record.file() {
            record.add_attribute("file", file.to_string());
        }
        if let Some(line) = log_record.line() {
            record.add_attribute("line", line);
        }

        struct KvExtractor<'a> {
            record: &'a mut LogRecord,
        }

        impl<'a, 'kvs> log::kv::Visitor<'kvs> for KvExtractor<'a> {
            fn visit_pair(
                &mut self,
                key: log::kv::Key<'kvs>,
                value: log::kv::Value<'kvs>,
            ) -> Result<(), log::kv::Error> {
                self.record
                    .add_attribute(key.to_string(), value.to_string());
                Ok(())
            }
        }

        let mut extractor = KvExtractor {
            record: &mut record,
        };
        log_record.key_values().visit(&mut extractor).ok();

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
            eprintln!("failed to flush logger {}: {}", self.name, err);
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
