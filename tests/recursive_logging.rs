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

use logforth::append;
use logforth::append::rolling_file::NonBlockingBuilder;
use logforth::append::rolling_file::RollingFileWriter;
use logforth::append::rolling_file::Rotation;
use logforth::layout;

// ensure logforth's impl doesn't properly handle recursive logging
#[test]
fn test_meta_logging_in_format_works() {
    let rolling = RollingFileWriter::builder()
        .rotation(Rotation::Minutely)
        .filename_prefix("example")
        .filename_suffix("log")
        .max_log_files(10)
        .max_file_size(1024 * 1024)
        .build("logs")
        .unwrap();
    let (writer, _guard) = NonBlockingBuilder::default().finish(rolling);

    let layout = |src: &'static str| {
        layout::CustomLayout::new(move |record| {
            Ok(format!("{src} [{}] {}", record.level(), record.args()).into_bytes())
        })
    };

    logforth::builder()
        .dispatch(|d| d.append(append::Stdout::default().with_layout(layout("out"))))
        .dispatch(|d| d.append(append::Stderr::default().with_layout(layout("err"))))
        .dispatch(|d| d.append(append::RollingFile::new(writer).with_layout(layout("file"))))
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
