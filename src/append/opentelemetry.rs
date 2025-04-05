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

//! Appenders and utilities for integrating with OpenTelemetry.

use std::borrow::Cow;
use std::time::SystemTime;

use log::Record;
use opentelemetry::logs::AnyValue;
use opentelemetry::logs::LogRecord;
use opentelemetry::logs::Logger;
use opentelemetry::logs::LoggerProvider;
use opentelemetry::InstrumentationScope;
use opentelemetry_otlp::LogExporter;
use opentelemetry_otlp::Protocol;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::logs::SdkLogRecord;
use opentelemetry_sdk::logs::SdkLoggerProvider;

use crate::append::Append;
use crate::diagnostic::Visitor;
use crate::Diagnostic;
use crate::Layout;

/// Specifies the wire protocol to use when sending logs to OpenTelemetry.
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
    layout: Option<Box<dyn Layout>>,
}

impl OpentelemetryLogBuilder {
    /// Creates a new [`OpentelemetryLogBuilder`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append::opentelemetry::OpentelemetryLogBuilder;
    ///
    /// let builder = OpentelemetryLogBuilder::new("my_service", "http://localhost:4317");
    /// ```
    pub fn new(name: impl Into<String>, otlp_endpoint: impl Into<String>) -> Self {
        OpentelemetryLogBuilder {
            name: name.into(),
            endpoint: otlp_endpoint.into(),
            protocol: Protocol::Grpc,
            labels: vec![],
            layout: None,
        }
    }

    /// Sets the wire protocol to use.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append::opentelemetry::OpentelemetryLogBuilder;
    /// use logforth::append::opentelemetry::OpentelemetryWireProtocol;
    ///
    /// let builder = OpentelemetryLogBuilder::new("my_service", "http://localhost:4317");
    /// builder.protocol(OpentelemetryWireProtocol::HttpJson);
    /// ```
    pub fn protocol(mut self, protocol: OpentelemetryWireProtocol) -> Self {
        self.protocol = match protocol {
            OpentelemetryWireProtocol::Grpc => Protocol::Grpc,
            OpentelemetryWireProtocol::HttpBinary => Protocol::HttpBinary,
            OpentelemetryWireProtocol::HttpJson => Protocol::HttpJson,
        };
        self
    }

    /// Adds a label to the logs.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append::opentelemetry::OpentelemetryLogBuilder;
    ///
    /// let builder = OpentelemetryLogBuilder::new("my_service", "http://localhost:4317");
    /// builder.label("env", "production");
    /// ```
    pub fn label(
        mut self,
        key: impl Into<Cow<'static, str>>,
        value: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.labels.push((key.into(), value.into()));
        self
    }

    /// Adds multiple labels to the logs.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append::opentelemetry::OpentelemetryLogBuilder;
    ///
    /// let builder = OpentelemetryLogBuilder::new("my_service", "http://localhost:4317");
    /// builder.labels(vec![("env", "production"), ("version", "1.0")]);
    /// ```
    pub fn labels<K, V>(mut self, labels: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<Cow<'static, str>>,
        V: Into<Cow<'static, str>>,
    {
        self.labels
            .extend(labels.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }

    /// Sets the layout for the logs.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append::opentelemetry::OpentelemetryLogBuilder;
    /// use logforth::layout::JsonLayout;
    ///
    /// let builder = OpentelemetryLogBuilder::new("my_service", "http://localhost:4317");
    /// builder.layout(JsonLayout::default());
    /// ```
    pub fn layout(mut self, layout: impl Into<Box<dyn Layout>>) -> Self {
        self.layout = Some(layout.into());
        self
    }

    /// Builds the [`OpentelemetryLog`] appender.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::append::opentelemetry::OpentelemetryLogBuilder;
    ///
    /// let builder = OpentelemetryLogBuilder::new("my_service", "http://localhost:4317");
    /// let otlp_appender = tokio::runtime::Runtime::new()
    ///     .unwrap()
    ///     .block_on(async { builder.build().unwrap() });
    /// ```
    pub fn build(self) -> Result<OpentelemetryLog, opentelemetry_otlp::ExporterBuildError> {
        let OpentelemetryLogBuilder {
            name,
            endpoint,
            protocol,
            labels,
            layout,
        } = self;

        let exporter = match protocol {
            Protocol::Grpc => LogExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint)
                .with_protocol(protocol)
                .build(),
            Protocol::HttpBinary | Protocol::HttpJson => LogExporter::builder()
                .with_http()
                .with_endpoint(endpoint)
                .with_protocol(protocol)
                .build(),
        }?;

        let resource = opentelemetry_sdk::Resource::builder()
            .with_attributes(
                labels
                    .into_iter()
                    .map(|(key, value)| opentelemetry::KeyValue::new(key, value)),
            )
            .build();

        let provider = SdkLoggerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(resource)
            .build();

        let library = InstrumentationScope::builder(name).build();
        let logger = provider.logger_with_scope(library);
        Ok(OpentelemetryLog {
            layout,
            logger,
            provider,
        })
    }
}

/// An appender that sends log records to OpenTelemetry.
///
/// # Examples
///
/// ```
/// use logforth::append::opentelemetry::OpentelemetryLogBuilder;
/// use logforth::append::opentelemetry::OpentelemetryWireProtocol;
///
/// let otlp_appender = tokio::runtime::Runtime::new().unwrap().block_on(async {
///     OpentelemetryLogBuilder::new("service_name", "http://localhost:4317")
///         .protocol(OpentelemetryWireProtocol::Grpc)
///         .build()
///         .unwrap();
/// });
/// ```
#[derive(Debug)]
pub struct OpentelemetryLog {
    layout: Option<Box<dyn Layout>>,
    logger: opentelemetry_sdk::logs::SdkLogger,
    provider: SdkLoggerProvider,
}

impl Append for OpentelemetryLog {
    fn append(&self, record: &Record, diagnostics: &[Box<dyn Diagnostic>]) -> anyhow::Result<()> {
        let mut log_record = self.logger.create_log_record();
        log_record.set_observed_timestamp(SystemTime::now());
        log_record.set_severity_number(log_level_to_otel_severity(record.level()));
        log_record.set_severity_text(record.level().as_str());
        log_record.set_target(record.target().to_string());
        log_record.set_body(AnyValue::Bytes(Box::new(match self.layout.as_ref() {
            None => record.args().to_string().into_bytes(),
            Some(layout) => layout.format(record, diagnostics)?,
        })));

        if let Some(module_path) = record.module_path() {
            log_record.add_attribute("module_path", module_path.to_string());
        }
        if let Some(file) = record.file() {
            log_record.add_attribute("file", file.to_string());
        }
        if let Some(line) = record.line() {
            log_record.add_attribute("line", line);
        }

        let mut extractor = KvExtractor {
            record: &mut log_record,
        };
        record.key_values().visit(&mut extractor)?;
        for d in diagnostics {
            d.visit(&mut extractor)?;
        }

        self.logger.emit(log_record);
        Ok(())
    }

    fn flush(&self) -> anyhow::Result<()> {
        self.provider.force_flush()?;
        Ok(())
    }
}

fn log_level_to_otel_severity(level: log::Level) -> opentelemetry::logs::Severity {
    match level {
        log::Level::Error => opentelemetry::logs::Severity::Error,
        log::Level::Warn => opentelemetry::logs::Severity::Warn,
        log::Level::Info => opentelemetry::logs::Severity::Info,
        log::Level::Debug => opentelemetry::logs::Severity::Debug,
        log::Level::Trace => opentelemetry::logs::Severity::Trace,
    }
}

struct KvExtractor<'a> {
    record: &'a mut SdkLogRecord,
}

impl<'kvs> log::kv::VisitSource<'kvs> for KvExtractor<'_> {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        let key = key.to_string();
        let value = value.to_string();
        self.record.add_attribute(key, value);
        Ok(())
    }
}

impl Visitor for KvExtractor<'_> {
    fn visit(&mut self, key: Cow<str>, value: Cow<str>) -> anyhow::Result<()> {
        let key = key.into_owned();
        let value = value.into_owned();
        self.record.add_attribute(key, value);
        Ok(())
    }
}
