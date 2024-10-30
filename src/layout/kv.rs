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

/// A helper struct to format log's key-value pairs.
///
/// This is useful when you want to display log's key-value pairs in a log message.
pub struct KvDisplay<'kvs> {
    kv: &'kvs dyn log::kv::Source,
}

impl<'kvs> KvDisplay<'kvs> {
    pub fn new(kv: &'kvs dyn log::kv::Source) -> Self {
        Self { kv }
    }
}

impl std::fmt::Display for KvDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut visitor = KvWriter { writer: f };
        self.kv.visit(&mut visitor).ok();
        Ok(())
    }
}

struct KvWriter<'a, 'kvs> {
    writer: &'kvs mut std::fmt::Formatter<'a>,
}

impl<'kvs> log::kv::Visitor<'kvs> for KvWriter<'_, 'kvs> {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        write!(self.writer, " {key}={value}")?;
        Ok(())
    }
}

/// A helper to collect log's key-value pairs.
///
/// This is useful when you want to collect log's key-value pairs for further processing.
pub fn collect_kvs(kv: &dyn log::kv::Source) -> Vec<(String, String)> {
    let mut collector = KvCollector { kv: Vec::new() };
    kv.visit(&mut collector).ok();
    collector.kv
}

struct KvCollector {
    kv: Vec<(String, String)>,
}

impl<'kvs> log::kv::Visitor<'kvs> for KvCollector {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        self.kv.push((key.to_string(), value.to_string()));
        Ok(())
    }
}
