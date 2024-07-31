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

use crate::error::LogError;

pub(crate) fn log_fallibly<F>(record: &log::Record, f: F)
where
    F: FnOnce(&log::Record) -> Result<(), LogError>,
{
    let Err(error) = f(record) else { return };

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
