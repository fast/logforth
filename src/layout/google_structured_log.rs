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
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt::Arguments;

use log::Record;
use serde::Serialize;
use serde_json::Value;

use crate::diagnostic::Visitor;
use crate::layout::Layout;
use crate::Diagnostic;

/// A layout for Google Cloud structured JSON logging.
///
/// See the [Google documentation](https://cloud.google.com/logging/docs/structured-logging) for more
/// information about the structure of the format.
///
/// Example format:
///
/// ```json
/// {"severity":"INFO","timestamp":"2025-04-02T10:34:33.225602Z","message":"Hello label value!","logging.googleapis.com/labels":{"label1":"this is a label value"},"logging.googleapis.com/trace":"projects/project-id/traces/612b91406b684ece2c4137ce0f3fd668", "logging.googleapis.com/sourceLocation":{"file":"examples/google_structured_log.rs","line":64,"function":"google_structured_log"}}
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
/// use logforth::layout::GoogleStructuredLogLayout;
///
/// let structured_json_layout = GoogleStructuredLogLayout::default();
/// ```
#[derive(Debug, Clone)]
pub struct GoogleStructuredLogLayout {
    trace_project_id: Option<String>,
    label_keys: BTreeSet<String>,

    // Heuristic keys to extract trace, spanId and traceSampled info from diagnostics.
    // These are currently hardcoded but may be customizable in the future.
    trace_keys: BTreeSet<String>,
    span_id_keys: BTreeSet<String>,
    trace_sampled_keys: BTreeSet<String>,
}

impl Default for GoogleStructuredLogLayout {
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

impl GoogleStructuredLogLayout {
    /// Sets the trace project ID for traces.
    ///
    /// If set, the trace_id, span_id, and trace_sampled fields will be set in the log record, in
    /// such a way that they can be linked to traces in the Google Cloud Trace service.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::layout::GoogleStructuredLogLayout;
    ///
    /// let structured_json_layout =
    ///     GoogleStructuredLogLayout::default().trace_project_id("project-id");
    /// ```
    pub fn trace_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.trace_project_id = Some(project_id.into());
        self
    }

    /// Extends the set of keys that should be treated as labels.
    ///
    /// Any key found in a log entry, and referenced here, will be stored in the labels field rather
    /// than the payload. Labels are indexed by default, but can only store strings.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::layout::GoogleStructuredLogLayout;
    ///
    /// let structured_json_layout =
    ///     GoogleStructuredLogLayout::default().label_keys(["label1", "label2"]);
    /// ```
    pub fn label_keys(mut self, label_keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let label_keys = label_keys.into_iter().map(Into::into);
        self.label_keys.extend(label_keys);
        self
    }
}

struct KvCollector<'a> {
    layout: &'a GoogleStructuredLogLayout,

    payload_fields: BTreeMap<String, Value>,
    labels: BTreeMap<String, Value>,
    trace: Option<String>,
    span_id: Option<String>,
    trace_sampled: Option<bool>,
}

impl<'kvs> log::kv::VisitSource<'kvs> for KvCollector<'kvs> {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        let key = key.to_string();
        if self.layout.label_keys.contains(&key) {
            self.labels.insert(key, value.to_string().into());
        } else {
            match serde_json::to_value(&value) {
                Ok(value) => self.payload_fields.insert(key, value),
                Err(_) => self.payload_fields.insert(key, value.to_string().into()),
            };
        }
        Ok(())
    }
}

impl Visitor for KvCollector<'_> {
    fn visit(&mut self, key: Cow<str>, value: Cow<str>) -> anyhow::Result<()> {
        if let Some(trace_project_id) = self.layout.trace_project_id.as_ref() {
            if self.trace.is_none() && self.layout.trace_keys.contains(key.as_ref()) {
                self.trace = Some(format!("projects/{trace_project_id}/traces/{value}"));
                return Ok(());
            }

            if self.span_id.is_none() && self.layout.span_id_keys.contains(key.as_ref()) {
                self.span_id = Some(value.into_owned());
                return Ok(());
            }

            if self.trace_sampled.is_none() && self.layout.trace_sampled_keys.contains(key.as_ref())
            {
                if let Ok(v) = value.parse() {
                    self.trace_sampled = Some(v);
                    return Ok(());
                }
            }
        }

        let key = key.into_owned();
        let value = value.into_owned();
        if self.layout.label_keys.contains(&key) {
            self.labels.insert(key, value.into());
        } else {
            self.payload_fields.insert(key, value.into());
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
    extra_fields: BTreeMap<String, Value>,
    severity: &'a str,
    timestamp: jiff::Timestamp,
    #[serde(serialize_with = "serialize_args")]
    message: &'a Arguments<'a>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[serde(rename = "logging.googleapis.com/labels")]
    labels: BTreeMap<String, Value>,
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

fn serialize_args<S>(args: &Arguments, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.collect_str(args)
}

impl Layout for GoogleStructuredLogLayout {
    fn format(
        &self,
        record: &Record,
        diagnostics: &[Box<dyn Diagnostic>],
    ) -> anyhow::Result<Vec<u8>> {
        let mut visitor = KvCollector {
            layout: self,
            payload_fields: BTreeMap::new(),
            labels: BTreeMap::new(),
            trace: None,
            span_id: None,
            trace_sampled: None,
        };

        record.key_values().visit(&mut visitor)?;
        for d in diagnostics {
            d.visit(&mut visitor)?;
        }

        let record_line = RecordLine {
            extra_fields: visitor.payload_fields,
            timestamp: jiff::Timestamp::now(),
            severity: record.level().as_str(),
            message: record.args(),
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

        Ok(serde_json::to_vec(&record_line)?)
    }
}
