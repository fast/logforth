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

use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::ErrorSink;
use logforth_core::kv;
use logforth_core::kv::Visitor;

use crate::Task;
use crate::channel::Receiver;

pub(crate) struct Worker {
    appends: Vec<Box<dyn Append>>,
    receiver: Receiver<Task>,
    error_sink: Box<dyn ErrorSink>,
}

impl Worker {
    pub(crate) fn new(
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

    pub(crate) fn run(self) {
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
                        for append in appends.iter() {
                            if let Err(err) = append.append(&record, &diag) {
                                let err = Error::new("failed to append record").with_source(err);
                                error_sink.sink(&err);
                            }
                        }
                    });
                }
                Task::Flush { done } => {
                    for append in appends.iter() {
                        if let Err(err) = append.flush() {
                            let err = Error::new("failed to flush appender").with_source(err);
                            error_sink.sink(&err);
                        }
                    }
                    let _ = done.send(());
                }
            }
        }
    }
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
