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
use std::sync::mpsc::Receiver;
use std::time::SystemTime;

use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::Trap;
use logforth_core::kv;
use logforth_core::kv::KeyValues;
use logforth_core::kv::Visitor;
use logforth_core::record::Level;
use logforth_core::record::Record;

use crate::Task;

pub(crate) struct Worker {
    appends: Vec<Box<dyn Append>>,
    receiver: Receiver<Task>,
    trap: Box<dyn Trap>,
}

impl Worker {
    pub(crate) fn new(
        appends: Vec<Box<dyn Append>>,
        receiver: Receiver<Task>,
        trap: Box<dyn Trap>,
    ) -> Self {
        Self {
            appends,
            receiver,
            trap,
        }
    }

    pub(crate) fn run(self) {
        let Self {
            appends,
            receiver,
            trap,
        } = self;

        while let Ok(task) = receiver.recv() {
            match task {
                Task::Log { record, diags } => {
                    let diags: &[Box<dyn Diagnostic>] = if diags.is_empty() {
                        &[]
                    } else {
                        &[Box::new(OwnedDiagnostic(diags))]
                    };
                    let payload = format_args!("{}", record.payload);
                    let record = Record::builder()
                        .time(record.now)
                        .level(record.level)
                        .target(record.target.as_ref())
                        .module_path(record.module_path.as_deref())
                        .file(record.file.as_deref())
                        .line(record.line)
                        .column(record.column)
                        .payload(payload)
                        .key_values(KeyValues::from(record.kvs.as_slice()))
                        .build();
                    for append in appends.iter() {
                        if let Err(err) = append.append(&record, diags) {
                            let err = Error::new("failed to append record").with_source(err);
                            trap.trap(&err);
                        }
                    }
                }
                Task::Flush { done } => {
                    let mut error = None;
                    for append in appends.iter() {
                        if let Err(err) = append.flush() {
                            error = Some(
                                error
                                    .unwrap_or_else(|| Error::new("failed to flush appender"))
                                    .with_source(err),
                            );
                        }
                    }
                    let _ = done.send(error);
                }
            }
        }
    }
}

#[derive(Debug)]
struct OwnedDiagnostic(Vec<(kv::KeyOwned, kv::ValueOwned)>);

impl Diagnostic for OwnedDiagnostic {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        for (key, value) in &self.0 {
            visitor.visit(key.by_ref(), value.by_ref())?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RecordOwned {
    // the observed time
    now: SystemTime,

    // the metadata
    level: Level,
    target: Cow<'static, str>,
    module_path: Option<Cow<'static, str>>,
    file: Option<Cow<'static, str>>,
    line: Option<u32>,
    column: Option<u32>,

    // the payload
    payload: Cow<'static, str>,

    // structural logging
    kvs: Vec<(kv::KeyOwned, kv::ValueOwned)>,
}

impl RecordOwned {
    pub fn from_record(record: &Record) -> Self {
        RecordOwned {
            now: record.time(),
            level: record.level(),
            target: if let Some(target) = record.target_static() {
                Cow::Borrowed(target)
            } else {
                Cow::Owned(record.target().to_string())
            },
            module_path: if let Some(module_path) = record.module_path_static() {
                Some(Cow::Borrowed(module_path))
            } else {
                record.module_path().map(|s| Cow::Owned(s.to_string()))
            },
            file: if let Some(file) = record.file_static() {
                Some(Cow::Borrowed(file))
            } else {
                record.file().map(|s| Cow::Owned(s.to_string()))
            },
            line: record.line(),
            column: record.column(),
            payload: if let Some(payload) = record.payload_static() {
                Cow::Borrowed(payload)
            } else {
                Cow::Owned(record.payload().to_string())
            },
            kvs: record
                .key_values()
                .iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
        }
    }
}
