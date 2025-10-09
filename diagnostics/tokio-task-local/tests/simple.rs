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

//!  sss

#[test]
fn test() {
    use logforth_core::Diagnostic;
    use logforth_core::kv::Key;
    use logforth_core::kv::Value;
    use logforth_core::kv::Visitor;
    use logforth_diagnostic_tokio_task_local::FutureExt;
    use logforth_diagnostic_tokio_task_local::TaskLocalDiagnostic;
    use tokio::runtime::Runtime;

    struct PrintVisitor;

    impl Visitor for PrintVisitor {
        fn visit(&mut self, key: Key<'_>, value: Value) -> Result<(), logforth_core::Error> {
            println!("{}: {}", key.as_str(), value);
            Ok(())
        }
    }

    let rt = Runtime::new().unwrap();

    rt.block_on(
        async {
            let diag = TaskLocalDiagnostic::default();
            diag.visit(&mut PrintVisitor).unwrap();
        }
        .with_task_local_context([("user_id".to_string(), "42".to_string())])
        .with_task_local_context([("request_id".to_string(), "abc123".to_string())]),
    );
}
