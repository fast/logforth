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

use std::borrow::Cow;
use std::cell::Cell;
use std::cell::RefCell;
use std::future::Future;
use std::io;
use std::num::NonZeroU16;
use std::ops::Deref;
use std::pin::pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use logforth_core::Logger;
use logforth_core::builder;
use logforth_core::kv::LogValue;
use logforth_core::kv::Value;
use logforth_core::kv::ValueOwned;
use logforth_core::kv::ValueView;
use logforth_core::record::Level;
use logforth_core::record::LevelFilter;
use tests_integration::RecordingAppender;

#[no_implicit_prelude]
mod no_prelude {
    pub fn emit(logger: &::logforth_core::Logger) {
        ::logforth_core::info!(logger, "no prelude plain");
        ::logforth_core::info!(logger, ["k" = 1_i32, "s" = "ok"], "no prelude kv");
    }
}

struct LoggerWrapper(Logger);

impl Deref for LoggerWrapper {
    type Target = Logger;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct CustomNonCopy(String);

impl LogValue for CustomNonCopy {
    fn log_value(&self) -> Value<'_> {
        Value::str(self.0.as_str())
    }
}

#[derive(Clone, Copy)]
struct CustomCopy(u64);

impl LogValue for CustomCopy {
    fn log_value(&self) -> Value<'_> {
        Value::u64(self.0)
    }
}

// `Send + !Sync` type converted eagerly via `LogValue::log_value` without holding `&Counter`
// across `.await` points in other arguments.
struct Counter(Cell<u32>);

impl LogValue for Counter {
    fn log_value(&self) -> Value<'_> {
        Value::u64(u64::from(self.0.get()))
    }
}

#[test]
fn all_macro_levels_and_argument_forms_capture_callsite_location_and_fields() {
    let sink = RecordingAppender::default();
    let logger = builder().dispatch(|d| d.append(sink.clone())).build();
    let wrapped = LoggerWrapper(logger.clone());

    let expected_line = line!() + 1;
    logforth_core::trace!(logger, "trace {}", 1,);
    logforth_core::debug!(&logger, ["k" = "v",], "debug");
    logforth_core::info!(wrapped, ["only_kv" = 42],);
    logforth_core::warn!(logger, [], "empty kvs");
    logforth_core::error!(logger, []);
    logforth_core::fatal!(logger, ["fatal" = true], "fatal event");
    logforth_core::log!(logger, Level::Info3, ["custom" = 3], "custom level");

    // `fatal!` records severity metadata only and never triggers `flush`.
    assert_eq!(sink.flush_count(), 0);

    let events = sink.events();
    assert_eq!(events.len(), 7);

    events[0].record.with(|r| {
        assert_eq!(r.level(), Level::Trace);
        assert_eq!(r.payload().to_string(), "trace 1");
        assert_eq!(r.module_path_static(), Some(module_path!()));
        assert_eq!(r.file_static(), Some(file!()));
        assert_eq!(r.line(), Some(expected_line));
        assert!(r.column().is_some());
        assert!(r.key_values().is_empty());
    });

    assert_eq!(events[2].payload_static, Some(""));
    events[2].record.with(|r| {
        assert_eq!(r.level(), Level::Info);
        assert_eq!(r.payload_static(), Some(""));
        assert_eq!(r.payload().to_string(), "");
        let (k, v) = r.key_values().iter().next().unwrap();
        assert_eq!(k.to_static_str(), Some("only_kv"));
        assert_eq!(v.to_i64(), Some(42));
    });

    events[5].record.with(|r| {
        assert_eq!(r.level(), Level::Fatal);
        assert_eq!(r.payload().to_string(), "fatal event");
        assert_eq!(r.key_values().get("fatal").unwrap().to_bool(), Some(true));
    });

    events[6].record.with(|r| {
        assert_eq!(r.level(), Level::Info3);
        assert_eq!(r.key_values().get("custom").unwrap().to_i64(), Some(3));
    });
}

#[test]
fn macro_evaluates_logger_and_level_once_and_skips_disabled_arguments() {
    let sink = RecordingAppender::default();
    let logger = builder()
        .dispatch(|d| {
            d.filter(LevelFilter::MoreSevereEqual(Level::Info))
                .append(sink.clone())
        })
        .dispatch(|d| {
            d.filter(LevelFilter::MoreSevereEqual(Level::Info))
                .append(sink.clone())
        })
        .build();

    let steps = RefCell::new(Vec::new());
    let get_logger = |tag: &'static str| {
        steps.borrow_mut().push(tag);
        &logger
    };
    let get_level = |tag: &'static str, level: Level| {
        steps.borrow_mut().push(tag);
        level
    };

    logforth_core::log!(
        get_logger("disabled_logger"),
        get_level("disabled_level", Level::Debug),
        ["kv" = {
            steps.borrow_mut().push("disabled_kv");
            1
        }],
        "msg {}",
        {
            steps.borrow_mut().push("disabled_arg");
            2
        },
    );
    assert_eq!(*steps.borrow(), ["disabled_logger", "disabled_level"]);
    assert!(sink.events().is_empty());

    steps.borrow_mut().clear();
    logforth_core::log!(
        get_logger("enabled_logger"),
        get_level("enabled_level", Level::Info),
        [
            "first" = {
                steps.borrow_mut().push("kv_1");
                10
            },
            "second" = {
                steps.borrow_mut().push("kv_2");
                20
            },
        ],
        "{} {}",
        {
            steps.borrow_mut().push("arg_1");
            "a"
        },
        {
            steps.borrow_mut().push("arg_2");
            "b"
        },
    );
    assert_eq!(
        *steps.borrow(),
        [
            "enabled_logger",
            "enabled_level",
            "kv_1",
            "kv_2",
            "arg_1",
            "arg_2"
        ]
    );
    assert_eq!(sink.events().len(), 2);
}

#[test]
fn macro_borrows_non_copy_locals_releases_level_guards_and_respects_hygiene() {
    trait Hijack: Sized {
        fn capture_val(&self) -> Value<'static> {
            Value::none()
        }
    }
    impl<T> Hijack for T {}

    const LOC: u32 = 9;

    let sink = RecordingAppender::default();
    let arc_logger = Arc::new(builder().dispatch(|d| d.append(sink.clone())).build());

    let owned_str = String::from("owned");
    let cow_bytes: Cow<'_, [u8]> = Cow::Borrowed(b"cb");
    let port_nz = NonZeroU16::new(8443).unwrap();
    let bytes_vec = vec![1_u8, 2, 3];
    let val_owned = ValueOwned::str("value_owned");
    let custom = CustomNonCopy(String::from("custom"));
    let opt_string = Some(String::from("opt_str"));
    let none_str: Option<String> = None;
    let mut local_static: &'static str = "before";
    let err = io::Error::other("boom");
    let mutex_level = Mutex::new(Level::Info);

    assert!(matches!(123_i32.capture_val().view(), ValueView::None));

    logforth_core::log!(
        arc_logger,
        *mutex_level.lock().unwrap(),
        [
            "owned_str" = owned_str,
            "cow_bytes" = cow_bytes,
            "port_nz" = port_nz,
            "bytes_vec" = bytes_vec,
            "val_owned" = val_owned,
            "temp_list" = ValueOwned::list([ValueOwned::i64(1), ValueOwned::i64(2)]),
            "custom" = custom,
            "custom_copy" = CustomCopy(42),
            "opt_string" = opt_string,
            "none_str" = none_str,
            "local_static" = local_static,
            "static_val" = Value::static_str("static_provenance"),
            "debug_err" = Value::debug(&err),
            "display_err" = Value::display(&err),
        ],
        "{LOC} {owned_str} unlocked={}",
        mutex_level.try_lock().is_ok(),
    );
    no_prelude::emit(&arc_logger);

    // Non-Copy locals remain unmoved and mutable borrows are released after the statement.
    local_static = "after";
    assert_eq!(local_static, "after");
    assert_eq!(owned_str, "owned");
    assert_eq!(&*cow_bytes, b"cb");
    assert_eq!(bytes_vec, [1, 2, 3]);
    assert_eq!(val_owned.view().to_str(), Some("value_owned"));
    assert_eq!(custom.0, "custom");
    assert_eq!(opt_string.as_deref(), Some("opt_str"));

    let events = sink.events();
    assert_eq!(events.len(), 3);
    events[0].record.with(|r| {
        assert_eq!(r.payload().to_string(), "9 owned unlocked=true");
        let kvs = r.key_values();
        assert_eq!(kvs.get("owned_str").unwrap().to_str(), Some("owned"));
        assert_eq!(kvs.get("cow_bytes").unwrap().to_bytes(), Some(&b"cb"[..]));
        assert_eq!(kvs.get("port_nz").unwrap().to_u64(), Some(8443));
        assert_eq!(kvs.get("custom").unwrap().to_str(), Some("custom"));
        assert_eq!(kvs.get("custom_copy").unwrap().to_u64(), Some(42));
        assert_eq!(kvs.get("opt_string").unwrap().to_str(), Some("opt_str"));
        assert!(matches!(kvs.get("none_str").unwrap(), ValueView::None));
        assert_eq!(
            kvs.get("static_val").unwrap().to_static_str(),
            Some("static_provenance")
        );
        assert_eq!(kvs.get("display_err").unwrap().to_string(), "boom");
    });
    events[2].record.with(|r| {
        assert_eq!(r.payload().to_string(), "no prelude kv");
        assert_eq!(r.key_values().get("k").unwrap().to_i64(), Some(1));
        assert_eq!(r.key_values().get("s").unwrap().to_str(), Some("ok"));
    });
}

#[test]
fn async_logging_across_await_points_remains_send_with_non_sync_temporaries() {
    fn assert_send_future<F: Future + Send + 'static>(f: F) -> F {
        f
    }

    async fn ready_val<T>(val: T) -> T {
        std::future::ready(val).await
    }

    let sink = RecordingAppender::default();
    let logger = builder().dispatch(|d| d.append(sink.clone())).build();

    let fut = assert_send_future(async move {
        let mutex_level = Mutex::new(Level::Info);
        let counter = Counter(Cell::new(5));
        let err = io::Error::other("async_err");

        logforth_core::log!(
            ready_val(logger).await,
            *mutex_level.lock().unwrap(),
            [
                "count" = counter,
                "step" = ready_val(String::from("one")).await,
                "err" = Value::display(&err),
                "next" = ready_val(2_u64).await,
            ],
            "completed {} unlocked={}",
            ready_val("ok").await,
            mutex_level.try_lock().is_ok(),
        );
        counter.0.set(6);
        counter.0.get()
    });

    let mut pinned = pin!(fut);
    let Poll::Ready(final_count) = pinned
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    else {
        panic!("ready future must complete in one poll");
    };
    assert_eq!(final_count, 6);

    let events = sink.events();
    assert_eq!(events.len(), 1);
    events[0].record.with(|r| {
        assert_eq!(r.payload().to_string(), "completed ok unlocked=true");
        assert_eq!(r.key_values().get("count").unwrap().to_u64(), Some(5));
        assert_eq!(r.key_values().get("step").unwrap().to_str(), Some("one"));
        assert_eq!(r.key_values().get("err").unwrap().to_string(), "async_err");
        assert_eq!(r.key_values().get("next").unwrap().to_u64(), Some(2));
    });
}
