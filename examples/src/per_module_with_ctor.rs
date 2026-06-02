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

use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::sync::RwLock;

use logforth::append::Stdout;
use logforth::filter::RustLogFilter;
use logforth::filter::rustlog::RustLogFilterBuilder;
use logforth::record::Level;
use logforth::starter_log;

static FILTER: OnceLock<Filter> = OnceLock::new();

mod foo {
    // This is how you can allow debug level logs for this module while keeping the default log
    // level for everything else at info.
    #[cfg(debug_assertions)]
    #[ctor::ctor]
    unsafe fn set_log_filter() {
        use logforth::record::Level;

        use crate::Filter;

        super::FILTER
            .get_or_init(|| Filter::new(Level::Info))
            .set_module_level(module_path!(), Level::Debug);
    }

    pub fn run() {
        log::debug!("foo debug");
        log::info!("foo info");
    }
}

mod bar {
    pub fn run() {
        log::debug!("bar debug");
        log::info!("bar info");
    }
}

fn main() {
    starter_log::builder()
        .dispatch(|dispatch_builder| {
            dispatch_builder
                .filter(
                    FILTER
                        .get_or_init(|| Filter::new(Level::Info))
                        .build_rustlog_filter(),
                )
                .append(Stdout::default())
        })
        .apply();

    foo::run();
    bar::run();
}

#[derive(Debug)]
struct Filter {
    default_level: Level,
    module_levels: RwLock<BTreeMap<String, Level>>,
}

impl Filter {
    fn new(default_level: Level) -> Self {
        Self {
            default_level,
            module_levels: RwLock::new(BTreeMap::new()),
        }
    }

    fn set_module_level(&self, module_path: &str, level: Level) {
        let mut module_levels = self
            .module_levels
            .write()
            .expect("filter write is poisoned");
        module_levels.insert(module_path.to_string(), level);
    }

    pub fn build_rustlog_filter(&self) -> RustLogFilter {
        let module_levels = self.module_levels.read().expect("filter read is poisoned");

        let mut directives = vec![self.default_level.name().to_string()];

        for (module_path, level) in module_levels.iter() {
            directives.push(format!("{module_path}={}", level.name()));
        }

        RustLogFilterBuilder::from_spec(directives.join(",")).build()
    }
}
