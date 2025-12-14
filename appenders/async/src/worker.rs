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

use std::sync::Arc;

use crossbeam_channel::Receiver;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::Trap;
use logforth_core::kv;
use logforth_core::kv::Visitor;

use crate::Task;

pub(crate) struct Worker {
    receiver: Receiver<Task>,
    trap: Arc<dyn Trap>,
}

impl Worker {
    pub(crate) fn new(receiver: Receiver<Task>, trap: Arc<dyn Trap>) -> Self {
        Self { receiver, trap }
    }

    pub(crate) fn run(self) {
        let Self { receiver, trap } = self;

        while let Ok(task) = receiver.recv() {
            match task {
                Task::Log {
                    appends,
                    record,
                    diags,
                } => {
                    let diags: &[Box<dyn Diagnostic>] = if diags.is_empty() {
                        &[]
                    } else {
                        &[Box::new(OwnedDiagnostic(diags))]
                    };
                    let record = record.as_record();
                    for append in appends.iter() {
                        if let Err(err) = append.append(&record, diags) {
                            let err = Error::new("failed to append record").set_source(err);
                            trap.trap(&err);
                        }
                    }
                }
                Task::Flush { appends } => {
                    for append in appends.iter() {
                        if let Err(err) = append.flush() {
                            let err = Error::new("failed to flush").set_source(err);
                            trap.trap(&err);
                        }
                    }
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
