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

use logforth::append;
use logforth::bridge::log::LogBridge;
use logforth::diagnostic::StaticDiagnostic;
use logforth::filter::RustLogFilter;
use logforth::filter::rustlog::RustLogFilterBuilder;
use logforth::layout::JsonLayout;

#[derive(serde::Serialize)]
struct Snapshot {
    queue_depths: Vec<u64>,
    healthy: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut context = StaticDiagnostic::default();
    context.insert("service", "worker");
    let worker_filter = "off,worker=info".parse::<RustLogFilter>()?;
    let logger = logforth::builder()
        .dispatch(|d| {
            d.filter(RustLogFilterBuilder::from_default_env_or("info").build())
                .diagnostic(context.clone())
                .append(append::Stderr::default())
        })
        .dispatch(|d| {
            d.filter(worker_filter)
                .diagnostic(context.clone())
                .append(append::Stdout::default().with_layout(JsonLayout::default()))
        })
        .build();
    log::set_boxed_logger(Box::new(LogBridge::new(logger.clone())))?;
    log::set_max_level(log::LevelFilter::Trace);

    // A dedicated stream has independent filtering and output policy.
    let usage = logforth::builder()
        .dispatch(|d| {
            d.diagnostic(context)
                .append(append::Stdout::default().with_layout(JsonLayout::default()))
        })
        .build()
        .named("usage");
    let worker = logger.named("worker");
    let snapshot = Snapshot {
        queue_depths: vec![3, 5],
        healthy: true,
    };
    logforth::info!(worker, {
        "snapshot" => logforth::kv::serde(&snapshot),
        "completed" => 8u64,
    }, "batch complete");
    logforth::info!(usage, { "resource" => "worker", "units" => 8i64 });
    log::warn!("dependency connection interrupted");

    // Stop producers first; keep exporter runtimes alive through both flushes.
    logger.flush();
    usage.flush();
    Ok(())
}
