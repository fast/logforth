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

use crate::Diagnostic;
use crate::Filter;
use crate::Logger;
use crate::filter::FilterResult;
use crate::kv::Key;
use crate::kv::Value;
use crate::record::Level;
use crate::record::LevelFilter;
use crate::record::Metadata;
use crate::record::MetadataBuilder;
use crate::record::RecordBuilder;

impl From<log::LevelFilter> for LevelFilter {
    fn from(level: log::LevelFilter) -> Self {
        match level {
            log::LevelFilter::Off => Self::Off,
            log::LevelFilter::Error => Self::Error,
            log::LevelFilter::Warn => Self::Warn,
            log::LevelFilter::Info => Self::Info,
            log::LevelFilter::Debug => Self::Debug,
            log::LevelFilter::Trace => Self::Trace,
        }
    }
}

impl From<log::Level> for Level {
    fn from(level: log::Level) -> Self {
        match level {
            log::Level::Error => Self::Error,
            log::Level::Warn => Self::Warn,
            log::Level::Info => Self::Info,
            log::Level::Debug => Self::Debug,
            log::Level::Trace => Self::Trace,
        }
    }
}

impl Filter for log::LevelFilter {
    fn enabled(&self, metadata: &Metadata, _: &[Box<dyn Diagnostic>]) -> FilterResult {
        if metadata.level() <= LevelFilter::from(*self) {
            FilterResult::Neutral
        } else {
            FilterResult::Reject
        }
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let metadata = MetadataBuilder::default()
            .target(metadata.target())
            .level(metadata.level().into())
            .build();

        Logger::enabled(self, &metadata)
    }

    fn log(&self, record: &log::Record) {
        // basic fields
        let mut builder = RecordBuilder::default()
            .args(*record.args())
            .level(record.level().into())
            .target(record.target())
            .line(record.line());

        // optional static fields
        builder = if let Some(module_path) = record.module_path_static() {
            builder.module_path_static(module_path)
        } else {
            builder.module_path(record.module_path())
        };
        builder = if let Some(file) = record.file_static() {
            builder.file_static(file)
        } else {
            builder.file(record.file())
        };

        // key-values
        let mut kvs = Vec::new();

        struct KeyValueVisitor<'a, 'b> {
            kvs: &'b mut Vec<(log::kv::Key<'a>, log::kv::Value<'a>)>,
        }

        impl<'a, 'b> log::kv::VisitSource<'a> for KeyValueVisitor<'a, 'b> {
            fn visit_pair(
                &mut self,
                key: log::kv::Key<'a>,
                value: log::kv::Value<'a>,
            ) -> Result<(), log::kv::Error> {
                self.kvs.push((key, value));
                Ok(())
            }
        }

        let mut visitor = KeyValueVisitor { kvs: &mut kvs };
        record.key_values().visit(&mut visitor).unwrap();

        let mut new_kvs = Vec::with_capacity(kvs.len());
        for (k, v) in kvs.iter() {
            new_kvs.push((Key::from(k.as_str()), Value::from_sval2(v)));
        }
        builder = builder.key_values(new_kvs.as_slice());

        Logger::log(self, &builder.build());
    }

    fn flush(&self) {
        Logger::flush(self);
    }
}
