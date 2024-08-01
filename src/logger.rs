// Copyright 2024 tison <wander4096@gmail.com>
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

use std::io::Write;

use log::LevelFilter;
use log::Metadata;
use log::Record;

use crate::append::Append;
use crate::append::AppendImpl;

#[derive(Debug)]
pub struct Logger {
    appends: Vec<AppendImpl>,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Logger {
    pub fn new() -> Self {
        Self { appends: vec![] }
    }

    pub fn add_append(mut self, append: impl Into<AppendImpl>) -> Self {
        self.appends.push(append.into());
        self
    }

    pub fn apply(self) -> Result<(), log::SetLoggerError> {
        log::set_boxed_logger(Box::new(self))?;
        log::set_max_level(LevelFilter::Trace);
        Ok(())
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.appends.iter().any(|append| append.enabled(metadata))
    }

    fn log(&self, record: &Record) {
        for append in &self.appends {
            if let Err(err) = append.try_append(record) {
                handle_error(record, err);
            }
        }
    }

    fn flush(&self) {
        for append in &self.appends {
            append.flush();
        }
    }
}

fn handle_error(record: &Record, error: anyhow::Error) {
    let Err(fallback_error) = write!(
        std::io::stderr(),
        r#"
            Error perform logging.
                Attempted to log: {args}
                Record: {record:?}
                Error: {error}
            "#,
        args = record.args(),
        record = record,
        error = error,
    ) else {
        return;
    };

    panic!(
        r#"
            Error performing stderr logging after error occurred during regular logging.
                Attempted to log: {args}
                Record: {record:?}
                Error: {error}
                Fallback error: {fallback_error}
            "#,
        args = record.args(),
        record = record,
        error = error,
        fallback_error = fallback_error,
    );
}
