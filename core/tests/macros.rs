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

use std::cell::Cell;
use std::fmt;
use std::sync::Arc;
use std::sync::Mutex;

use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::Logger;
use logforth_core::kv::KeyView;
use logforth_core::kv::ToValue;
use logforth_core::kv::Value;
use logforth_core::kv::ValueView;
use logforth_core::record::Level;
use logforth_core::record::LevelFilter;
use logforth_core::record::Record;

#[derive(Debug, PartialEq)]
enum CapturedValue {
    None,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    I128(i128),
    U128(u128),
    Char(char),
    String(String),
    Bytes(Vec<u8>),
    Debug(String),
    Display(String),
    Other(String),
}

impl CapturedValue {
    fn from_view(value: ValueView<'_>) -> Self {
        match value {
            ValueView::None => CapturedValue::None,
            ValueView::BorrowedStr(value) | ValueView::StaticStr(value) => {
                CapturedValue::String(value.to_owned())
            }
            ValueView::Bytes(value) => CapturedValue::Bytes(value.to_vec()),
            ValueView::Bool(value) => CapturedValue::Bool(value),
            ValueView::I64(value) => CapturedValue::I64(value),
            ValueView::U64(value) => CapturedValue::U64(value),
            ValueView::F64(value) => CapturedValue::F64(value),
            ValueView::I128(value) => CapturedValue::I128(value),
            ValueView::U128(value) => CapturedValue::U128(value),
            ValueView::Char(value) => CapturedValue::Char(value),
            ValueView::Debug(value) => CapturedValue::Debug(format!("{value:?}")),
            ValueView::Display(value) => CapturedValue::Display(format!("{value}")),
            value => CapturedValue::Other(format!("{value:?}")),
        }
    }
}

#[derive(Debug, PartialEq)]
struct CapturedRecord {
    level: Level,
    target: String,
    target_static: Option<String>,
    module_path: Option<String>,
    file: Option<String>,
    line: Option<u32>,
    column: Option<u32>,
    payload: String,
    key_values: Vec<(String, CapturedValue)>,
}

impl CapturedRecord {
    fn from_record(record: &Record<'_>) -> Result<Self, Error> {
        let mut key_values = Vec::new();
        record
            .key_values()
            .visit(&mut |key: KeyView<'_>, value: ValueView<'_>| {
                key_values.push((key.as_str().to_owned(), CapturedValue::from_view(value)));
                Ok(())
            })?;

        Ok(Self {
            level: record.level(),
            target: record.target().to_owned(),
            target_static: record.target_static().map(str::to_owned),
            module_path: record.module_path().map(str::to_owned),
            file: record.file().map(str::to_owned),
            line: record.line(),
            column: record.column(),
            payload: record.payload().to_string(),
            key_values,
        })
    }
}

#[derive(Clone, Debug, Default)]
struct Capture {
    records: Arc<Mutex<Vec<CapturedRecord>>>,
}

impl Capture {
    fn take(&self) -> Vec<CapturedRecord> {
        std::mem::take(&mut *self.records.lock().unwrap())
    }
}

impl Append for Capture {
    fn append(&self, record: &Record<'_>, _: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
        self.records
            .lock()
            .unwrap()
            .push(CapturedRecord::from_record(record)?);
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        Ok(())
    }
}

fn make_logger(capture: Capture) -> Logger {
    logforth_core::builder()
        .dispatch(|dispatch| dispatch.append(capture))
        .build()
}

fn make_filtered_logger(capture: Capture) -> Logger {
    logforth_core::builder()
        .dispatch(|dispatch| {
            dispatch
                .filter(LevelFilter::MoreSevereEqual(Level::Error))
                .append(capture)
        })
        .build()
}

#[test]
fn captures_fine_grained_level_metadata_and_typed_fields() {
    struct DebugOnly(u8);

    impl fmt::Debug for DebugOnly {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "DebugOnly({})", self.0)
        }
    }

    struct DisplayOnly(u8);

    impl fmt::Display for DisplayOnly {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "display({})", self.0)
        }
    }

    struct CustomValue(u8);

    impl ToValue for CustomValue {
        fn to_value(&self) -> Value<'_> {
            Value::u64(self.0.into())
        }
    }

    let capture = Capture::default();
    let logger = make_logger(capture.clone());
    let shorthand = 7_u32;
    let text = String::from("hello");
    let absent: Option<u64> = None;
    let dynamic_key = String::from("dynamic.key");
    let expected_line = line!() + 1;
    logforth_core::log!(
        logger,
        Level::Info2,
        shorthand,
        signed = -2_i32,
        unsigned = 3_usize,
        wide_signed = -4_i128,
        wide_unsigned = 5_u128,
        float = 1.5_f32,
        character = 'x',
        text = text,
        absent = absent,
        bytes = &b"bytes"[..],
        "literal.key" = true,
        (dynamic_key.as_str()) = 9_u8,
        debug:? = DebugOnly(10),
        display:% = DisplayOnly(11),
        custom = CustomValue(13);
        "accepted {}",
        12
    );

    let records = capture.take();
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.level, Level::Info2);
    assert_eq!(record.target, "macros");
    assert_eq!(record.target_static.as_deref(), Some("macros"));
    assert_eq!(record.module_path.as_deref(), Some("macros"));
    assert!(
        std::path::Path::new(record.file.as_deref().unwrap())
            .ends_with(std::path::Path::new("tests").join("macros.rs"))
    );
    assert_eq!(record.line, Some(expected_line));
    assert!(record.column.unwrap() > 0);
    assert_eq!(record.payload, "accepted 12");
    assert_eq!(
        record.key_values,
        [
            ("shorthand".to_owned(), CapturedValue::U64(7)),
            ("signed".to_owned(), CapturedValue::I64(-2)),
            ("unsigned".to_owned(), CapturedValue::U64(3)),
            ("wide_signed".to_owned(), CapturedValue::I128(-4)),
            ("wide_unsigned".to_owned(), CapturedValue::U128(5)),
            ("float".to_owned(), CapturedValue::F64(1.5)),
            ("character".to_owned(), CapturedValue::Char('x')),
            ("text".to_owned(), CapturedValue::String("hello".to_owned())),
            ("absent".to_owned(), CapturedValue::None),
            ("bytes".to_owned(), CapturedValue::Bytes(b"bytes".to_vec())),
            ("literal.key".to_owned(), CapturedValue::Bool(true)),
            ("dynamic.key".to_owned(), CapturedValue::U64(9)),
            (
                "debug".to_owned(),
                CapturedValue::Debug("DebugOnly(10)".to_owned())
            ),
            (
                "display".to_owned(),
                CapturedValue::Display("display(11)".to_owned())
            ),
            ("custom".to_owned(), CapturedValue::U64(13)),
        ]
    );
}

#[test]
fn named_logger_changes_target_without_hiding_source_module() {
    let capture = Capture::default();
    let logger = logforth_core::builder()
        .name("metering")
        .dispatch(|dispatch| dispatch.append(capture.clone()))
        .build();

    logforth_core::info!(logger, metering_kind = "compute";);

    let records = capture.take();
    assert_eq!(logger.name(), Some("metering"));
    assert_eq!(records[0].target, "metering");
    assert_eq!(records[0].target_static.as_deref(), Some("metering"));
    assert_eq!(records[0].module_path.as_deref(), Some("macros"));
}

#[test]
fn convenience_macros_cover_standard_levels() {
    let capture = Capture::default();
    let logger = Arc::new(make_logger(capture.clone()));

    logforth_core::fatal!(logger, "fatal");
    logforth_core::error!(logger, "error");
    logforth_core::warn!(&logger, "warn");
    logforth_core::info!(logger, "info");
    logforth_core::debug!(logger, "debug");
    logforth_core::trace!(logger, "trace");

    let records = capture.take();
    assert_eq!(
        records
            .iter()
            .map(|record| (record.level, record.payload.as_str()))
            .collect::<Vec<_>>(),
        [
            (Level::Fatal, "fatal"),
            (Level::Error, "error"),
            (Level::Warn, "warn"),
            (Level::Info, "info"),
            (Level::Debug, "debug"),
            (Level::Trace, "trace"),
        ]
    );
    assert!(records.iter().all(|record| record.target == "macros"));
    assert!(
        records
            .iter()
            .all(|record| record.target_static.as_deref() == Some("macros"))
    );
}

#[test]
fn disabled_records_do_not_evaluate_payload_or_fields() {
    let capture = Capture::default();
    let logger = make_filtered_logger(capture.clone());
    let evaluations = Cell::new(0);
    let expensive = || {
        evaluations.set(evaluations.get() + 1);
        42_u64
    };

    logforth_core::info!(
        logger,
        value = expensive();
        "value is {}",
        expensive()
    );

    assert_eq!(evaluations.get(), 0);
    assert!(capture.take().is_empty());
}

#[test]
fn macro_inputs_are_evaluated_once() {
    let capture = Capture::default();
    let logger = make_logger(capture.clone());
    let logger_evaluations = Cell::new(0);
    let level_evaluations = Cell::new(0);

    let logger_expression = || {
        logger_evaluations.set(logger_evaluations.get() + 1);
        &logger
    };
    let level_expression = || {
        level_evaluations.set(level_evaluations.get() + 1);
        Level::Debug3
    };
    logforth_core::log!(logger_expression(), level_expression(), "once");

    assert_eq!(logger_evaluations.get(), 1);
    assert_eq!(level_evaluations.get(), 1);
    assert_eq!(capture.take()[0].level, Level::Debug3);
}

#[test]
fn structured_record_may_omit_message() {
    let capture = Capture::default();
    let logger = make_logger(capture.clone());

    logforth_core::info!(logger, answer = 42_u64;);

    let records = capture.take();
    assert_eq!(records[0].payload, "");
    assert_eq!(
        records[0].key_values,
        [("answer".to_owned(), CapturedValue::U64(42))]
    );
}
