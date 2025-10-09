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

//! A diagnostic that enriches log records with tokio task-local context.

use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::kv::Key;
use logforth_core::kv::Value;
use logforth_core::kv::Visitor;
use tokio::task_local;

task_local! {
    static CONTEXT: Vec<(String, String)>;
}

/// A diagnostic that enriches log records with tokio task-local context.
#[derive(Default, Debug, Clone, Copy)]
#[non_exhaustive]
pub struct TaskLocalDiagnostic {}

impl Diagnostic for TaskLocalDiagnostic {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        CONTEXT.with(|map| {
            for (k, v) in map {
                let key = Key::new_ref(k.as_str());
                let value = Value::from(v);
                visitor.visit(key, value)?;
            }
            Ok(())
        })
    }
}

/// blablabla
pub trait FutureExt: Future {
    /// Run a future with a task-local context.
    fn with_task_local_context(
        self,
        kvs: impl IntoIterator<Item = (String, String)>,
    ) -> impl Future<Output = Self::Output>
    where
        Self: Sized,
        Self::Output: 'static,
    {
        let mut context = CONTEXT.try_with(|v| v.clone()).unwrap_or_default();
        context.extend(kvs);
        CONTEXT.scope(context, self)
    }
}

impl<F: Future> FutureExt for F {}
