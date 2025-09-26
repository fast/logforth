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

use std::num::NonZeroUsize;

use log::Record;
use logforth::Diagnostic;
use logforth::Error;
use logforth::Layout;
use logforth::append;
use logforth::append::file::FileBuilder;

#[derive(Debug)]
struct CustomLayout(&'static str);

impl Layout for CustomLayout {
    fn format(&self, record: &Record, diags: &[Box<dyn Diagnostic>]) -> Result<Vec<u8>, Error> {
        let _ = diags;
        Ok(format!("{} [{}] {}", self.0, record.level(), record.args()).into_bytes())
    }
}

// ensure logforth's impl doesn't properly handle recursive logging
#[test]
fn test_meta_logging_in_format_works() {
    let stdout = append::Stdout::default().with_layout(CustomLayout("out"));
    let stderr = append::Stderr::default().with_layout(CustomLayout("err"));
    let rolling = FileBuilder::new("logs", "example")
        .layout(CustomLayout("file"))
        .rollover_minutely()
        .rollover_size(NonZeroUsize::new(1024 * 1024).unwrap())
        .filename_suffix("log")
        .max_log_files(NonZeroUsize::new(10).unwrap())
        .build()
        .unwrap();

    logforth::builder()
        .dispatch(|d| d.append(stdout))
        .dispatch(|d| d.append(stderr))
        .dispatch(|d| d.append(rolling))
        .apply();

    struct Thing<'a>(&'a str);

    impl std::fmt::Display for Thing<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            log::debug!("formatting wrapping ({})", self.0);
            f.write_str(self.0)
        }
    }

    log::info!("I'm logging {}!", Thing("aha"));
}
