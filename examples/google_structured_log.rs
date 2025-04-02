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

use fastrace::collector::Config;
use fastrace::collector::ConsoleReporter;
use fastrace::collector::SpanContext;
use fastrace::prelude::{SpanId, TraceId};
use fastrace::Span;
use logforth::layout::GoogleStructuredLogLayout;
use logforth::{append, diagnostic};
use serde::Serialize;

#[derive(Serialize)]
struct NestedStructuredValue {
    c: String,
    d: bool,
}

#[derive(Serialize)]
struct MyStructuredValue {
    a: i32,
    b: NestedStructuredValue,
}

fn main() {
    logforth::builder()
        .dispatch(|d| {
            d.diagnostic(diagnostic::FastraceDiagnostic::default())
                .append(
                    append::Stdout::default().with_layout(
                        GoogleStructuredLogLayout::default()
                            .trace_project_id("project-id")
                            .label_keys(["label1"]),
                    ),
                )
        })
        .apply();

    fastrace::set_reporter(ConsoleReporter, Config::default());

    {
        let root = Span::root("root", SpanContext::new(TraceId::random(), SpanId(0)));
        let _g = root.set_local_parent();

        let structured_value = MyStructuredValue {
            a: 1,
            b: NestedStructuredValue {
                c: "Hello".into(),
                d: true,
            },
        };

        log::info!(label1="this is a label value", notLabel:serde=structured_value; "Hello label value!");
    }
}
