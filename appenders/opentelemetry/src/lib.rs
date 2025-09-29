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

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use std::borrow::Cow;
use std::fmt;
use std::time::SystemTime;

use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::Layout;
use logforth_core::append::Append;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::kv::Visitor;
use logforth_core::record::Level;
use logforth_core::record::Record;
use opentelemetry::InstrumentationScope;
use opentelemetry::logs::AnyValue;
use opentelemetry::logs::LogRecord;
use opentelemetry::logs::Logger;
use opentelemetry::logs::LoggerProvider;
use opentelemetry_otlp::LogExporter;
use opentelemetry_sdk::logs::SdkLogRecord;
use opentelemetry_sdk::logs::SdkLoggerProvider;

/// A builder to configure and create an [`OpentelemetryLog`] appender.
#[derive(Debug)]
pub struct OpentelemetryLogBuilder {
    name: String,
    log_exporter: LogExporter,
    labels: Vec<(Cow<'static, str>, Cow<'static, str>)>,
    make_body: Option<Box<dyn MakeBody>>,
}

impl OpentelemetryLogBuilder {
    /// Creates a new [`OpentelemetryLogBuilder`].
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_append_opentelemetry::OpentelemetryLogBuilder;
    /// use opentelemetry_otlp::LogExporter;
    /// use opentelemetry_otlp::WithExportConfig;
    ///
    /// let log_exporter = LogExporter::builder()
    ///     .with_http()
    ///     .with_endpoint("http://localhost:4317")
    ///     .build()
    ///     .unwrap();
    /// let builder = OpentelemetryLogBuilder::new("my_service", log_exporter);
    /// ```
    pub fn new(name: impl Into<String>, log_exporter: impl Into<LogExporter>) -> Self {
        OpentelemetryLogBuilder {
            name: name.into(),
            log_exporter: log_exporter.into(),
            labels: vec![],
            make_body: None,
        }
    }

    /// Adds a label to the logs.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_append_opentelemetry::OpentelemetryLogBuilder;
    /// use opentelemetry_otlp::LogExporter;
    /// use opentelemetry_otlp::WithExportConfig;
    ///
    /// let log_exporter = LogExporter::builder()
    ///     .with_http()
    ///     .with_endpoint("http://localhost:4317")
    ///     .build()
    ///     .unwrap();
    /// let builder = OpentelemetryLogBuilder::new("my_service", log_exporter);
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
    /// use logforth_append_opentelemetry::OpentelemetryLogBuilder;
    /// use opentelemetry_otlp::LogExporter;
    /// use opentelemetry_otlp::WithExportConfig;
    ///
    /// let log_exporter = LogExporter::builder()
    ///     .with_http()
    ///     .with_endpoint("http://localhost:4317")
    ///     .build()
    ///     .unwrap();
    /// let builder = OpentelemetryLogBuilder::new("my_service", log_exporter);
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

    /// Set the layout for the logs.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_append_opentelemetry::MakeBodyLayout;
    /// use logforth_append_opentelemetry::OpentelemetryLogBuilder;
    /// use logforth_layout_json::JsonLayout;
    /// use opentelemetry_otlp::LogExporter;
    /// use opentelemetry_otlp::WithExportConfig;
    ///
    /// let log_exporter = LogExporter::builder()
    ///     .with_http()
    ///     .with_endpoint("http://localhost:4317")
    ///     .build()
    ///     .unwrap();
    /// let builder = OpentelemetryLogBuilder::new("my_service", log_exporter);
    /// builder.make_body(MakeBodyLayout::new(JsonLayout::default()));
    /// ```
    pub fn make_body(mut self, make_body: impl Into<Box<dyn MakeBody>>) -> Self {
        self.make_body = Some(make_body.into());
        self
    }

    /// Builds the [`OpentelemetryLog`] appender.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_append_opentelemetry::OpentelemetryLogBuilder;
    /// use opentelemetry_otlp::LogExporter;
    /// use opentelemetry_otlp::WithExportConfig;
    ///
    /// let log_exporter = LogExporter::builder()
    ///     .with_http()
    ///     .with_endpoint("http://localhost:4317")
    ///     .build()
    ///     .unwrap();
    /// let builder = OpentelemetryLogBuilder::new("my_service", log_exporter);
    /// let otlp_appender = builder.build();
    /// ```
    pub fn build(self) -> OpentelemetryLog {
        let OpentelemetryLogBuilder {
            name,
            log_exporter,
            labels,
            make_body,
        } = self;

        let resource = opentelemetry_sdk::Resource::builder()
            .with_attributes(
                labels
                    .into_iter()
                    .map(|(key, value)| opentelemetry::KeyValue::new(key, value)),
            )
            .build();

        let provider = SdkLoggerProvider::builder()
            .with_batch_exporter(log_exporter)
            .with_resource(resource)
            .build();

        let library = InstrumentationScope::builder(name).build();

        let logger = provider.logger_with_scope(library);

        OpentelemetryLog {
            make_body,
            logger,
            provider,
        }
    }
}

/// An appender that sends log records to OpenTelemetry.
///
/// # Examples
///
/// ```
/// use logforth_append_opentelemetry::OpentelemetryLogBuilder;
/// use opentelemetry_otlp::LogExporter;
/// use opentelemetry_otlp::WithExportConfig;
///
/// let log_exporter = LogExporter::builder()
///     .with_http()
///     .with_endpoint("http://localhost:4317")
///     .build()
///     .unwrap();
/// let otlp_appender = OpentelemetryLogBuilder::new("service_name", log_exporter).build();
/// ```
#[derive(Debug)]
pub struct OpentelemetryLog {
    make_body: Option<Box<dyn MakeBody>>,
    logger: opentelemetry_sdk::logs::SdkLogger,
    provider: SdkLoggerProvider,
}

impl Append for OpentelemetryLog {
    fn append(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
        let now = SystemTime::now();

        let mut log_record = self.logger.create_log_record();
        log_record.set_timestamp(now);
        log_record.set_observed_timestamp(now);
        log_record.set_severity_number(log_level_to_otel_severity(record.level()));
        log_record.set_severity_text(record.level().as_str());
        log_record.set_target(record.target().to_string());
        log_record.set_body(match self.make_body.as_ref() {
            None => AnyValue::String(record.payload().to_string().into()),
            Some(make_body) => make_body.create(record, diags)?,
        });

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
        for d in diags {
            d.visit(&mut extractor)?;
        }

        self.logger.emit(log_record);
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        self.provider
            .force_flush()
            .map_err(|err| Error::new("failed to flush records").set_source(err))
    }
}

fn log_level_to_otel_severity(level: Level) -> opentelemetry::logs::Severity {
    match level {
        Level::Error => opentelemetry::logs::Severity::Error,
        Level::Warn => opentelemetry::logs::Severity::Warn,
        Level::Info => opentelemetry::logs::Severity::Info,
        Level::Debug => opentelemetry::logs::Severity::Debug,
        Level::Trace => opentelemetry::logs::Severity::Trace,
    }
}

/// A trait for formatting log records into a body that can be sent to OpenTelemetry.
pub trait MakeBody: fmt::Debug + Send + Sync + 'static {
    /// Creates a log record with optional diagnostics.
    fn create(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<AnyValue, Error>;
}

impl<T: MakeBody> From<T> for Box<dyn MakeBody> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

/// Make an OpenTelemetry body with the configured [`Layout`].
#[derive(Debug)]
pub struct MakeBodyLayout {
    layout: Box<dyn Layout>,
}

impl MakeBodyLayout {
    /// Creates a new `MakeBodyLayout` with the given layout.
    pub fn new(layout: impl Into<Box<dyn Layout>>) -> Self {
        MakeBodyLayout {
            layout: layout.into(),
        }
    }
}

impl MakeBody for MakeBodyLayout {
    fn create(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<AnyValue, Error> {
        let body = self.layout.format(record, diags)?;
        Ok(AnyValue::Bytes(Box::new(body)))
    }
}

struct KvExtractor<'a> {
    record: &'a mut SdkLogRecord,
}

impl Visitor for KvExtractor<'_> {
    fn visit(&mut self, key: Key, value: Value) -> Result<(), Error> {
        let key = key.into_string();
        let value = value.to_string();
        self.record.add_attribute(key, value);
        Ok(())
    }
}
