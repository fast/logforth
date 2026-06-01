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

use fastrace::Span;
use fastrace::collector::Config;
use fastrace::collector::ConsoleReporter;
use fastrace::collector::SpanContext;
use fastrace::prelude::SpanId;
use fastrace::prelude::TraceId;
use logforth::append;
use logforth::diagnostic;
use logforth::layout::GoogleCloudLoggingLayout;
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

#[derive(Serialize)]
struct MyEmptyStructuredValue {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    vec: Vec<()>,
}

fn main() {
    logforth::starter_log::builder()
        .dispatch(|d| {
            d.diagnostic(diagnostic::FastraceDiagnostic::default())
                .append(
                    append::Stdout::default().with_layout(
                        GoogleCloudLoggingLayout::default()
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

        log::info!(
            label1 = "this is a label value",
            notLabel:serde = structured_value;
            "Hello label value!",
        );

        let empty_value = MyEmptyStructuredValue { vec: vec![] };
        log::info!(empty_value:serde; "Hello empty value!");
        let non_empty_value = MyEmptyStructuredValue { vec: vec![()] };
        log::info!(non_empty_value:serde; "Hello non-empty value!");
    }
}
