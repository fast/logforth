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

//! Filtering for log records.

mod filter;
mod parser;

pub use filter::EnvFilter;
pub use filter::EnvFilterBuilder;
use log::{Level, LevelFilter};
pub use parser::ParseError;
use parser::parse_spec;
use std::fmt;

#[derive(Debug)]
struct FilterOp {
    filter: regex::Regex,
}

impl FilterOp {
    fn new(spec: &str) -> Result<Self, String> {
        match regex::Regex::new(spec) {
            Ok(filter) => Ok(Self { filter }),
            Err(err) => Err(err.to_string()),
        }
    }

    fn is_match(&self, s: &str) -> bool {
        self.filter.is_match(s)
    }
}

impl fmt::Display for FilterOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.filter.fmt(f)
    }
}

#[derive(Debug)]
struct Directive {
    name: Option<String>,
    level: LevelFilter,
}

// Check whether a level and target are enabled by the set of directives.
fn enabled(directives: &[Directive], level: Level, target: &str) -> bool {
    // Search for the longest match, the vector is assumed to be pre-sorted.
    for directive in directives.iter().rev() {
        match directive.name {
            Some(ref name) if !target.starts_with(&**name) => {}
            Some(..) | None => return level <= directive.level,
        }
    }
    false
}
