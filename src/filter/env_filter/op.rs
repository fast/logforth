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

use std::fmt;

#[derive(Debug)]
pub(crate) struct FilterOp {
    filter: regex::Regex,
}

impl FilterOp {
    pub(crate) fn new(spec: &str) -> Result<Self, String> {
        match regex::Regex::new(spec) {
            Ok(filter) => Ok(Self { filter }),
            Err(err) => Err(err.to_string()),
        }
    }

    pub(crate) fn is_match(&self, s: &str) -> bool {
        self.filter.is_match(s)
    }
}

impl fmt::Display for FilterOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.filter.fmt(f)
    }
}
