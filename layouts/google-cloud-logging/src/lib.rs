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

//! Layout for Google Cloud Structured Logging.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::kv::Visitor;
use logforth_core::layout::Layout;
use logforth_core::record::Record;
use serde::Serialize;

/// A layout for Google Cloud Structured Logging.
///
/// See the [Google documentation](https://cloud.google.com/logging/docs/structured-logging) for more
/// information about the structure of the format.
///
/// Example format:
///
/// ```json
/// {"severity":"INFO","timestamp":"2025-04-02T10:34:33.225602Z","message":"Hello label value!","logging.googleapis.com/labels":{"label1":"this is a label value"},"logging.googleapis.com/trace":"projects/project-id/traces/612b91406b684ece2c4137ce0f3fd668", "logging.googleapis.com/sourceLocation":{"file":"examples/google_cloud_logging","line":64,"function":"main"}}
/// ```
///
/// If the trace project ID is set, a few keys are treated specially:
/// - `trace_id`: Combined with trace project ID, set as the `logging.googleapis.com/trace` field.
/// - `span_id`: Set as the `logging.googleapis.com/spanId` field.
/// - `trace_sampled`: Set as the `logging.googleapis.com/trace_sampled` field.
///
/// Information may be stored either in the payload, or as labels. The payload allows a structured
/// value to be stored, but is not indexed by default. Labels are indexed by default, but can only
/// store strings.
///
/// # Examples
///
/// ```
/// use logforth_layout_google_cloud_logging::GoogleCloudLoggingLayout;
///
/// let layout = GoogleCloudLoggingLayout::default();
/// ```
#[derive(Debug, Clone)]
pub struct GoogleCloudLoggingLayout {
    trace_project_id: Option<String>,
    label_keys: BTreeSet<String>,

    // Heuristic keys to extract trace, spanId and traceSampled info from diagnostics.
    // These are currently hardcoded but may be customizable in the future.
    trace_keys: BTreeSet<String>,
    span_id_keys: BTreeSet<String>,
    trace_sampled_keys: BTreeSet<String>,
}

impl Default for GoogleCloudLoggingLayout {
    fn default() -> Self {
        Self {
            trace_project_id: None,
            label_keys: BTreeSet::new(),

            trace_keys: BTreeSet::from(["trace_id".to_string()]),
            span_id_keys: BTreeSet::from(["span_id".to_string()]),
            trace_sampled_keys: BTreeSet::from([
                "sampled".to_string(),
                "trace_sampled".to_string(),
            ]),
        }
    }
}

impl GoogleCloudLoggingLayout {
    /// Set the trace project ID for traces.
    ///
    /// If set, the trace_id, span_id, and trace_sampled fields will be set in the log record, in
    /// such a way that they can be linked to traces in the Google Cloud Trace service.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_layout_google_cloud_logging::GoogleCloudLoggingLayout;
    ///
    /// let layout = GoogleCloudLoggingLayout::default().trace_project_id("project-id");
    /// ```
    pub fn trace_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.trace_project_id = Some(project_id.into());
        self
    }

    /// Extend the set of keys that should be treated as labels.
    ///
    /// Any key found in a log entry, and referenced here, will be stored in the labels field rather
    /// than the payload. Labels are indexed by default, but can only store strings.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth_layout_google_cloud_logging::GoogleCloudLoggingLayout;
    ///
    /// let layout = GoogleCloudLoggingLayout::default().label_keys(["label1", "label2"]);
    /// ```
    pub fn label_keys(mut self, label_keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let label_keys = label_keys.into_iter().map(Into::into);
        self.label_keys.extend(label_keys);
        self
    }
}

struct KvCollector<'a> {
    layout: &'a GoogleCloudLoggingLayout,

    payload_fields: BTreeMap<String, serde_json::Value>,
    labels: BTreeMap<String, serde_json::Value>,
    trace: Option<String>,
    span_id: Option<String>,
    trace_sampled: Option<bool>,
}

impl Visitor for KvCollector<'_> {
    fn visit(&mut self, key: Key, value: Value) -> Result<(), Error> {
        let key = key.as_str();

        if let Some(trace_project_id) = self.layout.trace_project_id.as_ref() {
            if self.trace.is_none() && self.layout.trace_keys.contains(key) {
                self.trace = Some(format!("projects/{trace_project_id}/traces/{value}"));
                return Ok(());
            }

            if self.span_id.is_none() && self.layout.span_id_keys.contains(key) {
                self.span_id = Some(value.to_string());
                return Ok(());
            }

            if self.trace_sampled.is_none() && self.layout.trace_sampled_keys.contains(key) {
                self.trace_sampled = value.to_bool();
                return Ok(());
            }
        }

        let value = match serde_json::to_value(&value) {
            Ok(value) => value,
            Err(_) => value.to_string().into(),
        };

        if self.layout.label_keys.contains(key) {
            self.labels.insert(key.to_owned(), value);
        } else {
            self.payload_fields.insert(key.to_owned(), value);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
struct SourceLocation<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
struct RecordLine<'a> {
    #[serde(flatten)]
    extra_fields: BTreeMap<String, serde_json::Value>,
    severity: &'a str,
    timestamp: jiff::Timestamp,
    message: std::fmt::Arguments<'a>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[serde(rename = "logging.googleapis.com/labels")]
    labels: BTreeMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "logging.googleapis.com/trace")]
    trace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "logging.googleapis.com/spanId")]
    span_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "logging.googleapis.com/trace_sampled")]
    trace_sampled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "logging.googleapis.com/sourceLocation")]
    source_location: Option<SourceLocation<'a>>,
}

impl Layout for GoogleCloudLoggingLayout {
    fn format(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<Vec<u8>, Error> {
        // SAFETY: jiff::Timestamp::try_from only fails if the time is out of range, which is
        // very unlikely if the system clock is correct.
        let timestamp = jiff::Timestamp::try_from(record.time()).unwrap();

        let mut visitor = KvCollector {
            layout: self,
            payload_fields: BTreeMap::new(),
            labels: BTreeMap::new(),
            trace: None,
            span_id: None,
            trace_sampled: None,
        };

        record.key_values().visit(&mut visitor)?;
        for d in diags {
            d.visit(&mut visitor)?;
        }

        let record_line = RecordLine {
            extra_fields: visitor.payload_fields,
            timestamp,
            severity: record.level().name(),
            message: record.payload(),
            labels: visitor.labels,
            trace: visitor.trace,
            span_id: visitor.span_id,
            trace_sampled: visitor.trace_sampled,
            source_location: Some(SourceLocation {
                file: record.file(),
                line: record.line(),
                function: record.module_path(),
            }),
        };

        // SAFETY: RecordLine is serializable.
        Ok(serde_json::to_vec(&record_line).unwrap())
    }
}
