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

use std::borrow::Cow;
use std::io::Write;

use log::Metadata;
use log::Record;

use crate::append::utils::log_fallibly;
use crate::append::Append;
use crate::append::AppendImpl;

pub struct Stdout {
    pub stream: std::io::Stdout,
    pub sep: Cow<'static, str>,
}

impl Append for Stdout {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        log_fallibly(record, |record| {
            // TODO(tisonkun): avoid deadlock if formatting itself is logged
            write!(self.stream.lock(), "{}{}", record.args(), self.sep)?;
            Ok(())
        })
    }

    fn flush(&self) {
        let _ = self.stream.lock().flush();
    }
}

impl From<std::io::Stdout> for AppendImpl {
    fn from(stream: std::io::Stdout) -> Self {
        AppendImpl::Stdout(Stdout {
            stream,
            sep: Cow::Borrowed("\n"),
        })
    }
}
