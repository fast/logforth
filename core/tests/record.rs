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

use logforth_core::record::Level;
use logforth_core::record::Metadata;
use logforth_core::record::Record;

#[test]
fn record_preserves_supplied_metadata_across_owned_boundary() {
    let owned = {
        let target = String::from("service");
        let module_path = String::from("service::worker");
        let file = String::from("src/worker.rs");
        let metadata = Metadata::builder()
            .level(Level::Warn2)
            .target(&target)
            .module_path(Some(&module_path))
            .file(Some(&file))
            .line(Some(42))
            .column(Some(7))
            .build();
        let record = Record::builder()
            .metadata(metadata)
            .payload(format_args!("retrying"))
            .build();

        assert_eq!(record.metadata(), &metadata);
        assert_eq!(record.level(), Level::Warn2);
        assert_eq!(record.target(), "service");
        assert_eq!(record.module_path(), Some("service::worker"));
        assert_eq!(record.file(), Some("src/worker.rs"));
        assert_eq!(record.line(), Some(42));
        assert_eq!(record.column(), Some(7));

        record.to_owned()
    };

    owned.with(|record| {
        assert_eq!(record.level(), Level::Warn2);
        assert_eq!(record.target(), "service");
        assert_eq!(record.module_path(), Some("service::worker"));
        assert_eq!(record.file(), Some("src/worker.rs"));
        assert_eq!(record.line(), Some(42));
        assert_eq!(record.column(), Some(7));
        assert_eq!(record.payload().to_string(), "retrying");
    });
}
