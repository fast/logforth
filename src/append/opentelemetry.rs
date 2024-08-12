// Copyright 2024 CratesLand Developers
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
    Grpc,
    HttpBinary,
    HttpJson,
}

/// An appender that sends log records to opentelemetry.
#[derive(Debug)]
pub struct OpentelemetryLog {
    name: String,
    category: String,
    library: Arc<InstrumentationLibrary>,
    provider: LoggerProvider,
}

impl OpentelemetryLog {
    pub fn new(
        name: impl Into<String>,
        category: impl Into<String>,
        otlp_endpoint: impl Into<String>,
        protocol: OpentelemetryWireProtocol,
    ) -> Result<Self, opentelemetry::logs::LogError> {
        let name = name.into();
        let category = category.into();
        let otlp_endpoint = otlp_endpoint.into();

        let exporter = match protocol {
            OpentelemetryWireProtocol::Grpc => opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint)
                .with_protocol(Protocol::Grpc)
                .with_timeout(Duration::from_secs(
                    opentelemetry_otlp::OTEL_EXPORTER_OTLP_TIMEOUT_DEFAULT,
                ))
                .build_log_exporter(),
            OpentelemetryWireProtocol::HttpBinary => opentelemetry_otlp::new_exporter()
                .http()
                .with_endpoint(otlp_endpoint)
                .with_protocol(Protocol::HttpBinary)
                .with_timeout(Duration::from_secs(
                    opentelemetry_otlp::OTEL_EXPORTER_OTLP_TIMEOUT_DEFAULT,
                ))
                .build_log_exporter(),
            OpentelemetryWireProtocol::HttpJson => opentelemetry_otlp::new_exporter()
                .http()
                .with_endpoint(otlp_endpoint)
                .with_protocol(Protocol::HttpJson)
                .with_timeout(Duration::from_secs(
                    opentelemetry_otlp::OTEL_EXPORTER_OTLP_TIMEOUT_DEFAULT,
                ))
                .build_log_exporter(),
        }?;

        let provider = LoggerProvider::builder()
            .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
            .build();

        let library = Arc::new(InstrumentationLibrary::builder(name.clone()).build());

        Ok(Self {
            name,
            category,
            library,
            provider,
        })
    }
}

impl Append for OpentelemetryLog {
    fn append(&self, log_record: &Record) -> anyhow::Result<()> {
        let provider = self.provider.clone();
        let logger = provider.library_logger(self.library.clone());

        let mut record = LogRecord::default();
        record.observed_timestamp = Some(SystemTime::now());
        record.severity_number = Some(log_level_to_otel_severity(log_record.level()));
        record.severity_text = Some(log_record.level().as_str().into());
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
