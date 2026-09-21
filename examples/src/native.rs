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

use logforth::Level;
use logforth::LevelFilter;
use logforth::append;
use logforth::diagnostic::StaticDiagnostic;
use logforth::kv::Value;

fn main() {
    let mut context = StaticDiagnostic::default();
    context.insert("service", "worker");
    let logger = logforth::builder()
        .dispatch(|d| {
            d.filter(LevelFilter::MoreSevereEqual(Level::Info))
                .diagnostic(context)
                .append(append::Stderr::default())
        })
        .build();

    let queue = String::from("background");
    logforth::info!(logger, {
        "queue" => queue,
        "completed" => 3u64,
        "healthy" => true,
        "details" => Value::debug(&[1, 2, 3]),
    }, "batch complete");
    logforth::info!(logger, { "queue.depth" => 0 });
    logforth::debug!(logger, "this event is filtered out");

    // Stop producers before flushing, while appender resources are still alive.
    logger.flush();
}
