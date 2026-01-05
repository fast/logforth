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
use logforth::diagnostic;
use logforth::record::LevelFilter;

fn main() {
    logforth::starter_log::builder()
        .dispatch(|d| {
            d.filter(LevelFilter::All)
                .append(logforth::append::FastraceEvent::default())
        })
        .dispatch(|d| {
            d.diagnostic(diagnostic::FastraceDiagnostic::default())
                .append(logforth::append::Stderr::default())
        })
        .apply();

    fastrace::set_reporter(ConsoleReporter, Config::default());

    {
        let root = Span::root("root", SpanContext::random());
        let _g = root.set_local_parent();

        log::error!("Hello syslog error!");
        log::warn!("Hello syslog warn!");
        log::info!("Hello syslog info!");
        log::debug!("Hello syslog debug!");
        log::trace!("Hello syslog trace!");
    }

    fastrace::flush();
}
