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

use std::any::Any;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;

use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::ErrorSink;
use logforth_core::kv;
use logforth_core::kv::Visitor;

use crate::Task;
use crate::channel::Receiver;

pub struct Worker {
    appends: Vec<Box<dyn Append>>,
    receiver: Receiver<Task>,
    error_sink: Box<dyn ErrorSink>,
}

impl Worker {
    pub fn new(
        appends: Vec<Box<dyn Append>>,
        receiver: Receiver<Task>,
        error_sink: Box<dyn ErrorSink>,
    ) -> Self {
        Self {
            appends,
            receiver,
            error_sink,
        }
    }

    pub fn run(self) {
        let Self {
            appends,
            receiver,
            error_sink,
        } = self;

        while let Ok(task) = receiver.recv() {
            match task {
                Task::Log { record, diags } => {
                    let diag = AsyncDiagnostic(diags);

                    record.with(|record| {
                        for append in &appends {
                            run_guarded(&*error_sink, "failed to append record", || {
                                append.append(&record, &diag)
                            });
                        }
                    });
                }
                Task::Flush { done } => {
                    for append in &appends {
                        run_guarded(&*error_sink, "failed to flush appender", || append.flush());
                    }
                    let _ = done.send(());
                }
            }
        }
    }
}

fn run_guarded(
    error_sink: &dyn ErrorSink,
    context: &'static str,
    f: impl FnOnce() -> Result<(), Error>,
) {
    let result =
        catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|payload| Err(panic_error(payload)));
    if let Err(err) = result {
        let err = Error::new(context).with_source(err);
        error_sink.sink(&err);
    }
}

fn panic_error(payload: Box<dyn Any + Send>) -> Error {
    let msg = payload
        .downcast_ref::<&'static str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic payload");
    Error::new(format!("appender panicked: {msg}"))
}

#[derive(Debug)]
struct AsyncDiagnostic(Vec<(kv::KeyOwned, kv::ValueOwned)>);

impl Diagnostic for AsyncDiagnostic {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        for (key, value) in &self.0 {
            visitor.visit(key.view(), value.view())?;
        }
        Ok(())
    }
}
