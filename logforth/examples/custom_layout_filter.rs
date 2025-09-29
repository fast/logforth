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

//! An example of custom layout and filter.

use logforth::Diagnostic;
use logforth::Error;
use logforth::Filter;
use logforth::Layout;
use logforth::append;
use logforth::filter::FilterResult;
use logforth::record::Level;
use logforth::record::Metadata;
use logforth::record::Record;

#[derive(Debug)]
struct CustomFilter;

impl Filter for CustomFilter {
    fn enabled(&self, metadata: &Metadata, _: &[Box<dyn Diagnostic>]) -> FilterResult {
        if metadata.level() < Level::Info {
            FilterResult::Accept
        } else {
            FilterResult::Reject
        }
    }
}

#[derive(Debug)]
struct CustomLayout;

impl Layout for CustomLayout {
    fn format(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<Vec<u8>, Error> {
        let _ = diags;
        Ok(format!("[Alert] {}", record.payload()).into_bytes())
    }
}

fn main() {
    logforth::bridge::setup_log_crate();
    logforth::builder()
        .dispatch(|d| {
            d.filter(CustomFilter)
                .append(append::Stdout::default().with_layout(CustomLayout))
        })
        .apply();

    log::error!("Hello error!");
    log::warn!("Hello warn!");
    log::info!("Hello info!");
    log::debug!("Hello debug!");
    log::trace!("Hello trace!");
}
