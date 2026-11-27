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

use std::io;

use logforth::append;
use logforth::diagnostic::ThreadLocalDiagnostic;
use logforth::kv::Value;
use logforth::record::Level;
use logforth::record::LevelFilter;

fn main() {
    let logger = logforth::builder()
        .dispatch(|d| {
            d.filter(LevelFilter::All)
                .diagnostic(ThreadLocalDiagnostic::default())
                .append(append::Stdout::default())
        })
        .build();

    let addr = "127.0.0.1:8080";
    let err = io::Error::other("connection reset");

    let _outer_guard = ThreadLocalDiagnostic::scope([("request_id", "req-42"), ("attempt", "1")]);

    logforth::trace!(logger, ["step" = "poll"], "polling socket");
    logforth::debug!(logger, ["peer" = addr], "connecting to peer");
    logforth::info!(logger, ["port" = 8080, "tls" = true], "listening on {addr}");

    {
        let _inner_guard = ThreadLocalDiagnostic::scope([("attempt", "2")]);
        logforth::warn!(
            logger,
            ["queue_len" = 512],
            "backpressure threshold reached"
        );
    }

    logforth::error!(logger, ["error" = Value::display(&err)], "request failed");
    logforth::fatal!(
        logger,
        ["component" = "storage"],
        "unrecoverable state corruption"
    );
    logforth::log!(
        logger,
        Level::Info2,
        ["custom" = true],
        "custom severity level"
    );

    logger.flush();
}
