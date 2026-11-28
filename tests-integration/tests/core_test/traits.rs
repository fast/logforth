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

use std::cell::Cell;
use std::fmt;

use logforth_core::DispatchBuilder;
use logforth_core::Error;
use logforth_core::Logger;
use logforth_core::LoggerBuilder;
use logforth_core::append::Stderr;
use logforth_core::append::Stdout;
use logforth_core::append::Testing;
use logforth_core::diagnostic::StaticDiagnostic;
use logforth_core::diagnostic::ThreadLocalDiagnostic;
use logforth_core::error::BlackHoleErrorSink;
use logforth_core::error::StderrErrorSink;
use logforth_core::filter::FilterResult;
use logforth_core::kv::DebugValue;
use logforth_core::kv::DisplayValue;
use logforth_core::kv::Key;
use logforth_core::kv::KeyOwned;
use logforth_core::kv::KeyValues;
use logforth_core::kv::KeyValuesIter;
use logforth_core::kv::KeyView;
use logforth_core::kv::ListValue;
use logforth_core::kv::ListValueIter;
use logforth_core::kv::MapValue;
use logforth_core::kv::MapValueIter;
use logforth_core::kv::Value;
use logforth_core::kv::ValueOwned;
use logforth_core::kv::ValueView;
use logforth_core::layout::PlainTextLayout;
use logforth_core::record::Level;
use logforth_core::record::LevelFilter;
use logforth_core::record::Record;
use logforth_core::record::RecordBuilder;
use logforth_core::record::RecordOwned;

#[test]
fn public_types_are_send_sync_and_unpin() {
    fn assert_send_sync_unpin<T: Send + Sync + Unpin>() {}
    fn assert_unpin<T: Unpin>() {}

    assert_send_sync_unpin::<Logger>();
    assert_send_sync_unpin::<LoggerBuilder>();
    assert_send_sync_unpin::<DispatchBuilder>();
    assert_send_sync_unpin::<Error>();

    // `Record` and `RecordBuilder` carry stack-resident `fmt::Arguments<'_>` (`!Send + !Sync`),
    // while `RecordOwned` promotes events into a `Send + Sync + 'static` snapshot.
    assert_unpin::<Record<'_>>();
    assert_unpin::<RecordBuilder<'_>>();
    assert_send_sync_unpin::<RecordOwned>();
    assert_send_sync_unpin::<Level>();
    assert_send_sync_unpin::<LevelFilter>();
    assert_send_sync_unpin::<FilterResult>();

    assert_send_sync_unpin::<Key<'_>>();
    assert_send_sync_unpin::<KeyOwned>();
    assert_send_sync_unpin::<KeyView<'_>>();
    assert_send_sync_unpin::<Value<'_>>();
    assert_send_sync_unpin::<ValueOwned>();
    assert_send_sync_unpin::<KeyValues<'_>>();
    assert_send_sync_unpin::<KeyValuesIter<'_>>();
    assert_send_sync_unpin::<ListValue<'_>>();
    assert_send_sync_unpin::<ListValueIter<'_>>();
    assert_send_sync_unpin::<MapValue<'_>>();
    assert_send_sync_unpin::<MapValueIter<'_>>();

    assert_send_sync_unpin::<StaticDiagnostic>();
    assert_send_sync_unpin::<ThreadLocalDiagnostic>();
    assert_send_sync_unpin::<Stdout>();
    assert_send_sync_unpin::<Stderr>();
    assert_send_sync_unpin::<Testing>();
    assert_send_sync_unpin::<PlainTextLayout>();
    assert_send_sync_unpin::<StderrErrorSink>();
    assert_send_sync_unpin::<BlackHoleErrorSink>();
}

#[test]
fn hot_path_handles_and_zst_components_preserve_compact_memory_layout() {
    assert_eq!(size_of::<Logger>(), size_of::<usize>());
    assert_eq!(size_of::<Error>(), size_of::<usize>());
    assert_eq!(size_of::<Result<(), Error>>(), size_of::<usize>());
    assert_eq!(size_of::<Option<Level>>(), size_of::<u8>());

    assert_eq!(size_of::<PlainTextLayout>(), 0);
    assert_eq!(size_of::<ThreadLocalDiagnostic>(), 0);
    assert_eq!(size_of::<StderrErrorSink>(), 0);
    assert_eq!(size_of::<BlackHoleErrorSink>(), 0);
}

#[test]
fn debug_and_display_value_wrappers_accept_non_sync_local_formatters() {
    struct LocalCellFormatter(Cell<u32>);

    impl fmt::Debug for LocalCellFormatter {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let next = self.0.get() + 1;
            self.0.set(next);
            write!(f, "dbg:{next}")
        }
    }

    impl fmt::Display for LocalCellFormatter {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let next = self.0.get() + 10;
            self.0.set(next);
            write!(f, "disp:{next}")
        }
    }

    let local = LocalCellFormatter(Cell::new(0));
    let dbg_view = ValueView::Debug(DebugValue::new(&local));
    let disp_view = ValueView::Display(DisplayValue::new(&local));

    assert_eq!(dbg_view.to_string(), "dbg:1");
    assert_eq!(disp_view.to_string(), "disp:11");
    assert_eq!(dbg_view.to_owned().view().to_str(), Some("dbg:12"));
    assert_eq!(disp_view.to_owned().view().to_str(), Some("disp:22"));
}
