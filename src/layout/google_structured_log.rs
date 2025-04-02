// Copyright 2025 FastLabs Developers
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
use std::collections::{BTreeMap, HashSet};
use std::fmt::Arguments;

use crate::diagnostic::Visitor;
use crate::layout::Layout;
use crate::Diagnostic;
use jiff::Timestamp;
use log::Record;
use serde::Serialize;
use serde_json::Value;

/// A layout for Google Cloud structured JSON logging.
///
/// See the [Google documentation](https://cloud.google.com/logging/docs/structured-logging) for more
/// information about the structure of the format.
///
/// Example format:
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
#[derive(Default, Debug, Clone)]
pub struct GoogleStructuredLogLayout {
    trace_project_id: Option<String>,
    label_keys: Option<HashSet<String>>,
}

impl GoogleStructuredLogLayout {
    /// Sets the trace project ID for traces.
    ///
    /// If set, the trace_id, span_id, and trace_sampled fields will be set in the log record, in such
    /// a way that they can be linked to traces in the Google Cloud Trace service.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::layout::GoogleStructuredLogLayout;
    ///
    /// let structured_json_layout = GoogleStructuredLogLayout::default().trace_project_id("project-id");
    /// ```
    pub fn trace_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.trace_project_id = Some(project_id.into());
        self
    }

    /// Sets the set of keys that should be treated as labels.
    ///
    /// Any key found in a log entry, and referenced here, will be stored in the labels field rather than
    /// the payload. Labels are indexed by default, but can only store strings.
    ///
    /// # Examples
    ///
    /// ```
    /// use logforth::layout::GoogleStructuredLogLayout;
    ///
    /// let structured_json_layout = GoogleStructuredLogLayout::default().label_keys(["label1", "label2"]);
    /// ```
    pub fn label_keys(mut self, label_keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.label_keys = Some(label_keys.into_iter().map(Into::into).collect());
        self
    }
}

struct KvCollector<'a> {
    trace_project_id: Option<&'a str>,
    label_keys: Option<&'a HashSet<String>>,
    payload_fields: &'a mut BTreeMap<Cow<'a, str>, Value>,
    labels: &'a mut BTreeMap<Cow<'a, str>, Cow<'a, str>>,
    trace: &'a mut Option<String>,
    span_id: &'a mut Option<String>,
    trace_sampled: &'a mut Option<bool>,
}

impl<'kvs> log::kv::VisitSource<'kvs> for KvCollector<'kvs> {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        let k = key
            .to_borrowed_str()
            .map_or_else(|| key.to_string().into(), Cow::Borrowed);

        if self
            .label_keys
            .as_ref()
            .map_or(false, |keys| keys.contains(k.as_ref()))
        {
            self.labels.insert(k, value.to_string().into());
        } else {
            self.payload_fields.insert(
                k,
                serde_json::to_value(&value).unwrap_or(value.to_string().into()),
            );
        }

        Ok(())
    }
}

impl Visitor for KvCollector<'_> {
    fn visit(&mut self, key: Cow<str>, value: Cow<str>) {
        if let Some(trace_project_id) = self.trace_project_id.as_ref() {
            if key == "trace_id" {
                *self.trace = Some(format!("projects/{}/traces/{}", trace_project_id, value));
                return;
            }

            if key == "span_id" {
                *self.span_id = Some(value.into_owned());
                return;
            }

            if key == "trace_sampled" {
                if let Some(v) = value.parse().ok() {
                    *self.trace_sampled = Some(v);
                }
                return;
            }
        }

        if self
            .label_keys
            .as_ref()
            .map_or(false, |keys| keys.contains(value.as_ref()))
        {
            self.labels
                .insert(key.into_owned().into(), value.into_owned().into());
        } else {
            self.payload_fields
                .insert(key.into_owned().into(), value.into_owned().into());
        }
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
    extra_fields: &'a BTreeMap<Cow<'a, str>, Value>,
    severity: &'a str,
    timestamp: Timestamp,
    #[serde(serialize_with = "serialize_args")]
    message: &'a Arguments<'a>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[serde(rename = "logging.googleapis.com/labels")]
    labels: &'a BTreeMap<Cow<'a, str>, Cow<'a, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "logging.googleapis.com/trace")]
    trace: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "logging.googleapis.com/spanId")]
    span_id: Option<&'a str>,
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
        let mut payload_fields = BTreeMap::new();
        let mut labels = BTreeMap::new();
        let mut visitor = KvCollector {
            trace_project_id: self.trace_project_id.as_deref(),
            label_keys: self.label_keys.as_ref(),
            payload_fields: &mut payload_fields,
            labels: &mut labels,
            trace: &mut None,
            span_id: &mut None,
            trace_sampled: &mut None,
        };

        record.key_values().visit(&mut visitor)?;
        for d in diagnostics {
            d.visit(&mut visitor);
        }

        let record_line = RecordLine {
            extra_fields: visitor.payload_fields,
            timestamp: Timestamp::now(),
            severity: record.level().as_str(),
            message: record.args(),
            labels: visitor.labels,
            trace: visitor.trace.as_deref(),
            span_id: visitor.span_id.as_deref(),
            trace_sampled: visitor.trace_sampled.clone(),
            source_location: Some(SourceLocation {
                file: record.file(),
                line: record.line(),
                function: record.module_path(),
            }),
        };

        Ok(serde_json::to_vec(&record_line)?)
    }
}
