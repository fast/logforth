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

#[doc(hidden)]
pub mod __private {
    use std::fmt;

    use crate::Logger;
    use crate::kv::Key;
    use crate::kv::KeyValues;
    use crate::kv::Value;
    use crate::record::Level;
    use crate::record::Record;

    #[derive(Debug, Clone, Copy)]
    pub struct Location {
        module_path: &'static str,
        file: &'static str,
        line: u32,
        column: u32,
    }

    impl Location {
        pub const fn new(
            module_path: &'static str,
            file: &'static str,
            line: u32,
            column: u32,
        ) -> Self {
            Self {
                module_path,
                file,
                line,
                column,
            }
        }
    }

    pub fn log(
        logger: &Logger,
        level: Level,
        loc: &'static Location,
        kvs: &[(Key<'_>, Value<'_>)],
        payload: fmt::Arguments<'_>,
    ) {
        let record = Record::builder()
            .level(level)
            .module_path_static(loc.module_path)
            .file_static(loc.file)
            .line(Some(loc.line))
            .column(Some(loc.column))
            .key_values(KeyValues::from(kvs))
            .payload(payload)
            .build();
        logger.log(&record);
    }

    pub struct ValueRef<'a, T: ?Sized>(pub &'a T);

    impl<T: ?Sized> Copy for ValueRef<'_, T> {}

    impl<T: ?Sized> Clone for ValueRef<'_, T> {
        fn clone(&self) -> Self {
            *self
        }
    }

    pub trait CaptureRef<'v> {
        fn __logforth_capture(self) -> Value<'v>;
    }

    impl<'a, 'v, T: ?Sized> CaptureRef<'v> for &&ValueRef<'a, T>
    where
        &'a T: Into<Value<'v>>,
    {
        #[inline]
        fn __logforth_capture(self) -> Value<'v> {
            self.0.into()
        }
    }

    pub trait CaptureOptionRef<'v> {
        fn __logforth_capture(self) -> Value<'v>;
    }

    impl<'a, 'v, U> CaptureOptionRef<'v> for &ValueRef<'a, Option<U>>
    where
        &'a U: Into<Value<'v>>,
    {
        #[inline]
        fn __logforth_capture(self) -> Value<'v> {
            match self.0 {
                Some(v) => v.into(),
                None => Value::none(),
            }
        }
    }

    pub trait CaptureCopy<'v> {
        fn __logforth_capture(self) -> Value<'v>;
    }

    impl<'a, 'v, T> CaptureCopy<'v> for ValueRef<'a, T>
    where
        T: Copy + Into<Value<'v>>,
    {
        #[inline]
        fn __logforth_capture(self) -> Value<'v> {
            (*self.0).into()
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __log_impl {
    (@entry $logger:expr, $level:expr, [], $($arg:tt)+) => {{
        match &($logger) {
            __logforth_logger => {
                let __logforth_logger: &$crate::Logger = __logforth_logger;
                let __logforth_level: $crate::record::Level = $level;
                if __logforth_logger.enabled(__logforth_level) {
                    $crate::__private_log(
                        __logforth_logger,
                        __logforth_level,
                        {
                            static LOC: $crate::__private::Location =
                                $crate::__private::Location::new(
                                    ::core::module_path!(),
                                    ::core::file!(),
                                    ::core::line!(),
                                    ::core::column!(),
                                );
                            &LOC
                        },
                        &[],
                        ::core::format_args!($($arg)+),
                    );
                }
            }
        }
    }};
    (@entry $logger:expr, $level:expr, [$($kv:tt)+], $($arg:tt)+) => {{
        match &($logger) {
            __logforth_logger => {
                let __logforth_logger: &$crate::Logger = __logforth_logger;
                let __logforth_level: $crate::record::Level = $level;
                if __logforth_logger.enabled(__logforth_level) {
                    $crate::__log_impl!(
                        @kv [$($kv)+] -> [] (__logforth_logger, __logforth_level) $($arg)+
                    );
                }
            }
        }
    }};
    (@kv [] -> [$($kv:ident)+] ($logger:ident, $level:ident) $($arg:tt)+) => {{
        let __logforth_kvs = [$($kv),+];
        $crate::__private_log(
            $logger,
            $level,
            {
                static LOC: $crate::__private::Location = $crate::__private::Location::new(
                    ::core::module_path!(),
                    ::core::file!(),
                    ::core::line!(),
                    ::core::column!(),
                );
                &LOC
            },
            &__logforth_kvs,
            ::core::format_args!($($arg)+),
        );
    }};
    (@kv [$key:literal = $value:expr] -> [$($acc:tt)*] ($logger:ident, $level:ident) $($arg:tt)+) => {
        match &($value) {
            __logforth_val => {
                let __logforth_kv = (
                    $crate::kv::Key::new($key),
                    {
                        #[allow(unused_imports)]
                        use $crate::__private::{
                            CaptureCopy as _, CaptureOptionRef as _, CaptureRef as _,
                        };
                        (&&$crate::__private::ValueRef(__logforth_val)).__logforth_capture()
                    },
                );
                $crate::__log_impl!(
                    @kv [] -> [$($acc)* __logforth_kv] ($logger, $level) $($arg)+
                );
            }
        }
    };
    (@kv [$key:literal = $value:expr, $($rest:tt)*] -> [$($acc:tt)*] ($logger:ident, $level:ident) $($arg:tt)+) => {
        match &($value) {
            __logforth_val => {
                let __logforth_kv = (
                    $crate::kv::Key::new($key),
                    {
                        #[allow(unused_imports)]
                        use $crate::__private::{
                            CaptureCopy as _, CaptureOptionRef as _, CaptureRef as _,
                        };
                        (&&$crate::__private::ValueRef(__logforth_val)).__logforth_capture()
                    },
                );
                $crate::__log_impl!(
                    @kv [$($rest)*] -> [$($acc)* __logforth_kv] ($logger, $level) $($arg)+
                );
            }
        }
    };
}

/// Logs a record at the specified [`Level`](crate::record::Level) to a [`Logger`](crate::Logger).
///
/// `$logger` and `$level` are evaluated once up front. Structured key-values and format arguments
/// are evaluated at most once and only when [`Logger::enabled`](crate::Logger::enabled) returns
/// `true` for `$level`.
///
/// # Examples
///
/// ```
/// use logforth_core::Logger;
/// use logforth_core::kv::Value;
/// use logforth_core::record::Level;
///
/// let logger = Logger::noop();
/// let addr = "127.0.0.1:8080";
/// let err = std::io::Error::other("timeout");
///
/// logforth_core::log!(logger, Level::Info, "listening on {addr}");
/// logforth_core::log!(
///     logger,
///     Level::Info2,
///     ["port" = 8080, "tls" = true],
///     "server started"
/// );
/// logforth_core::log!(logger, Level::Warn, ["error" = Value::display(&err)]);
/// ```
#[macro_export]
macro_rules! log {
    ($logger:expr, $level:expr, [$($kv:tt)*] $(,)?) => {
        $crate::__log_impl!(@entry $logger, $level, [$($kv)*], "")
    };
    ($logger:expr, $level:expr, [$($kv:tt)*], $($arg:tt)+) => {
        $crate::__log_impl!(@entry $logger, $level, [$($kv)*], $($arg)+)
    };
    ($logger:expr, $level:expr, $($arg:tt)+) => {
        $crate::__log_impl!(@entry $logger, $level, [], $($arg)+)
    };
}

/// Logs a record at [`Level::Trace`](crate::record::Level::Trace) to a [`Logger`](crate::Logger).
///
/// # Examples
///
/// ```
/// use logforth_core::Logger;
///
/// let logger = Logger::noop();
/// logforth_core::trace!(logger, ["step" = "poll"], "polling socket");
/// ```
#[macro_export]
macro_rules! trace {
    ($logger:expr, [$($kv:tt)*] $(,)?) => {
        $crate::log!($logger, $crate::record::Level::Trace, [$($kv)*])
    };
    ($logger:expr, [$($kv:tt)*], $($arg:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Trace, [$($kv)*], $($arg)+)
    };
    ($logger:expr, $($arg:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Trace, $($arg)+)
    };
}

/// Logs a record at [`Level::Debug`](crate::record::Level::Debug) to a [`Logger`](crate::Logger).
///
/// # Examples
///
/// ```
/// use logforth_core::Logger;
///
/// let logger = Logger::noop();
/// logforth_core::debug!(logger, ["attempt" = 1], "connecting to peer");
/// ```
#[macro_export]
macro_rules! debug {
    ($logger:expr, [$($kv:tt)*] $(,)?) => {
        $crate::log!($logger, $crate::record::Level::Debug, [$($kv)*])
    };
    ($logger:expr, [$($kv:tt)*], $($arg:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Debug, [$($kv)*], $($arg)+)
    };
    ($logger:expr, $($arg:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Debug, $($arg)+)
    };
}

/// Logs a record at [`Level::Info`](crate::record::Level::Info) to a [`Logger`](crate::Logger).
///
/// # Examples
///
/// ```
/// use logforth_core::Logger;
///
/// let logger = Logger::noop();
/// logforth_core::info!(logger, ["port" = 8080, "tls" = true], "server started");
/// ```
#[macro_export]
macro_rules! info {
    ($logger:expr, [$($kv:tt)*] $(,)?) => {
        $crate::log!($logger, $crate::record::Level::Info, [$($kv)*])
    };
    ($logger:expr, [$($kv:tt)*], $($arg:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Info, [$($kv)*], $($arg)+)
    };
    ($logger:expr, $($arg:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Info, $($arg)+)
    };
}

/// Logs a record at [`Level::Warn`](crate::record::Level::Warn) to a [`Logger`](crate::Logger).
///
/// # Examples
///
/// ```
/// use logforth_core::Logger;
///
/// let logger = Logger::noop();
/// logforth_core::warn!(
///     logger,
///     ["queue_len" = 512],
///     "backpressure threshold reached"
/// );
/// ```
#[macro_export]
macro_rules! warn {
    ($logger:expr, [$($kv:tt)*] $(,)?) => {
        $crate::log!($logger, $crate::record::Level::Warn, [$($kv)*])
    };
    ($logger:expr, [$($kv:tt)*], $($arg:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Warn, [$($kv)*], $($arg)+)
    };
    ($logger:expr, $($arg:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Warn, $($arg)+)
    };
}

/// Logs a record at [`Level::Error`](crate::record::Level::Error) to a [`Logger`](crate::Logger).
///
/// # Examples
///
/// ```
/// use logforth_core::Logger;
/// use logforth_core::kv::Value;
///
/// let logger = Logger::noop();
/// let err = std::io::Error::other("connection reset");
/// logforth_core::error!(logger, ["error" = Value::debug(&err)], "request failed");
/// ```
#[macro_export]
macro_rules! error {
    ($logger:expr, [$($kv:tt)*] $(,)?) => {
        $crate::log!($logger, $crate::record::Level::Error, [$($kv)*])
    };
    ($logger:expr, [$($kv:tt)*], $($arg:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Error, [$($kv)*], $($arg)+)
    };
    ($logger:expr, $($arg:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Error, $($arg)+)
    };
}

/// Logs a record at [`Level::Fatal`](crate::record::Level::Fatal) to a [`Logger`](crate::Logger).
///
/// `Level::Fatal` records severity metadata only and neither flushes nor terminates the process.
///
/// # Examples
///
/// ```
/// use logforth_core::Logger;
///
/// let logger = Logger::noop();
/// logforth_core::fatal!(
///     logger,
///     ["component" = "storage"],
///     "unrecoverable state corruption"
/// );
/// ```
#[macro_export]
macro_rules! fatal {
    ($logger:expr, [$($kv:tt)*] $(,)?) => {
        $crate::log!($logger, $crate::record::Level::Fatal, [$($kv)*])
    };
    ($logger:expr, [$($kv:tt)*], $($arg:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Fatal, [$($kv)*], $($arg)+)
    };
    ($logger:expr, $($arg:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Fatal, $($arg)+)
    };
}

#[cfg(test)]
#[forbid(unsafe_code)]
mod tests {
    use std::borrow::Cow;
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::future::Future;
    use std::io;
    use std::ops::Range;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;

    use crate::Append;
    use crate::Diagnostic;
    use crate::Error;
    use crate::Logger;
    use crate::builder;
    use crate::kv::Value;
    use crate::kv::ValueOwned;
    use crate::record::Level;
    use crate::record::LevelFilter;
    use crate::record::Record;
    use crate::record::RecordOwned;

    #[no_implicit_prelude]
    mod no_prelude {
        pub(super) fn emit(logger: &crate::Logger) {
            crate::info!(logger, "no prelude plain");
            crate::info!(logger, ["k" = 1_i32, "s" = "ok"], "no prelude kv");
        }
    }

    #[derive(Debug)]
    struct CapturedRecord {
        owned: RecordOwned,
        payload_static: Option<&'static str>,
    }

    #[derive(Debug, Default, Clone)]
    struct Collector {
        records: Arc<Mutex<Vec<CapturedRecord>>>,
        flushes: Arc<AtomicUsize>,
    }

    impl Append for Collector {
        fn append(&self, record: &Record, _diags: &dyn Diagnostic) -> Result<(), Error> {
            self.records.lock().unwrap().push(CapturedRecord {
                owned: record.to_owned(),
                payload_static: record.payload_static(),
            });
            Ok(())
        }

        fn flush(&self) -> Result<(), Error> {
            self.flushes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct CustomNonCopy(String);

    impl<'a> From<&'a CustomNonCopy> for Value<'a> {
        fn from(v: &'a CustomNonCopy) -> Self {
            Value::str(v.0.as_str())
        }
    }

    #[derive(Clone, Copy)]
    struct CustomCopy(u64);

    impl From<CustomCopy> for Value<'_> {
        fn from(v: CustomCopy) -> Self {
            Value::u64(v.0)
        }
    }

    // Send + !Sync type converting eagerly into Value without holding &Counter across .await.
    struct Counter(Cell<u32>);

    impl From<&Counter> for Value<'_> {
        fn from(c: &Counter) -> Self {
            Value::u64(u64::from(c.0.get()))
        }
    }

    struct RangeVal(Range<usize>);

    impl From<&RangeVal> for Value<'_> {
        fn from(r: &RangeVal) -> Self {
            Value::u64((r.0.end - r.0.start) as u64)
        }
    }

    #[test]
    fn levels_syntax_forms_and_callsite_metadata() {
        let collector = Collector::default();
        let logger = builder().dispatch(|d| d.append(collector.clone())).build();

        let expected_line = line!() + 1;
        crate::trace!(logger, "trace {}", 1,);
        crate::debug!(logger, ["k" = "v",], "debug");
        crate::info!(logger, ["only_kv" = 42],);
        crate::warn!(logger, [], "empty kvs");
        crate::error!(logger, []);
        crate::fatal!(logger, ["fatal" = true], "fatal event");
        crate::log!(logger, Level::Info3, ["custom" = 3], "custom level");

        assert_eq!(collector.flushes.load(Ordering::SeqCst), 0);

        let captured = collector.records.lock().unwrap();
        assert_eq!(captured.len(), 7);

        captured[0].owned.with(|r| {
            assert_eq!(r.level(), Level::Trace);
            assert_eq!(r.payload().to_string(), "trace 1");
            assert_eq!(r.module_path_static(), Some(module_path!()));
            assert_eq!(r.file_static(), Some(file!()));
            assert_eq!(r.line(), Some(expected_line));
            assert!(r.column().is_some());
            assert!(r.key_values().is_empty());
        });

        assert_eq!(captured[2].payload_static, Some(""));
        captured[2].owned.with(|r| {
            assert_eq!(r.level(), Level::Info);
            assert_eq!(r.payload().to_string(), "");
            let (k, v) = r.key_values().iter().next().unwrap();
            assert_eq!(k.to_static_str(), Some("only_kv"));
            assert_eq!(v.to_i64(), Some(42));
        });

        captured[5].owned.with(|r| {
            assert_eq!(r.level(), Level::Fatal);
            assert_eq!(r.payload().to_string(), "fatal event");
            assert_eq!(r.key_values().get("fatal").unwrap().to_bool(), Some(true));
        });

        captured[6].owned.with(|r| {
            assert_eq!(r.level(), Level::Info3);
            assert_eq!(r.key_values().get("custom").unwrap().to_i64(), Some(3));
        });
    }

    #[test]
    fn evaluation_order_and_disabled_short_circuit() {
        let collector = Collector::default();
        let logger = builder()
            .dispatch(|d| {
                d.filter(LevelFilter::MoreSevereEqual(Level::Info))
                    .append(collector.clone())
            })
            .dispatch(|d| {
                d.filter(LevelFilter::MoreSevereEqual(Level::Info))
                    .append(collector.clone())
            })
            .build();

        let events = RefCell::new(Vec::new());
        let get_logger = |tag: &'static str| {
            events.borrow_mut().push(tag);
            &logger
        };
        let get_level = |tag: &'static str, level: Level| {
            events.borrow_mut().push(tag);
            level
        };

        crate::log!(
            get_logger("disabled_logger"),
            get_level("disabled_level", Level::Debug),
            ["kv" = {
                events.borrow_mut().push("disabled_kv");
                1
            }],
            "msg {}",
            {
                events.borrow_mut().push("disabled_arg");
                2
            },
        );
        assert_eq!(*events.borrow(), vec!["disabled_logger", "disabled_level"]);

        events.borrow_mut().clear();
        crate::log!(
            get_logger("enabled_logger"),
            get_level("enabled_level", Level::Info),
            [
                "first" = {
                    events.borrow_mut().push("kv_1");
                    10
                },
                "second" = {
                    events.borrow_mut().push("kv_2");
                    20
                },
            ],
            "{} {}",
            {
                events.borrow_mut().push("arg_1");
                "a"
            },
            {
                events.borrow_mut().push("arg_2");
                "b"
            },
        );
        assert_eq!(
            *events.borrow(),
            vec![
                "enabled_logger",
                "enabled_level",
                "kv_1",
                "kv_2",
                "arg_1",
                "arg_2",
            ]
        );
        assert_eq!(collector.records.lock().unwrap().len(), 2);
    }

    #[test]
    fn non_copy_locals_temporaries_and_mixed_lifetimes() {
        let collector = Collector::default();
        let arc_logger = Arc::new(builder().dispatch(|d| d.append(collector.clone())).build());

        let owned_str = String::from("owned");
        let cow_str: Cow<'_, str> = Cow::Owned(String::from("cow"));
        let bytes_vec = vec![1_u8, 2, 3];
        let bytes_arr = [4_u8, 5];
        let val_owned = ValueOwned::str("value_owned");
        let custom = CustomNonCopy(String::from("custom"));
        let opt_string = Some(String::from("opt_str"));
        let opt_val_owned = Some(ValueOwned::i64(99));
        let opt_custom = Some(CustomNonCopy(String::from("opt_custom")));
        let opt_u16: Option<u16> = Some(7);
        let opt_str_ref: Option<&str> = Some("opt_ref");
        let none_str: Option<String> = None;
        let none_ref: Option<&str> = None;
        let mut static_str: &'static str = "mutable_local_static_str";
        let static_val: Value<'static> = Value::static_str("static_val");
        let err = io::Error::other("boom");

        crate::info!(
            arc_logger,
            [
                "int_lit" = 8080,
                "float_lit" = 0.5,
                "owned_str" = owned_str,
                "owned_str_ref" = &owned_str,
                "cow_str" = cow_str,
                "bytes_vec" = bytes_vec,
                "bytes_slice" = bytes_vec.as_slice(),
                "bytes_arr" = bytes_arr,
                "val_owned" = val_owned,
                "val_owned_ref" = &val_owned,
                "temp_val_owned" = ValueOwned::list([ValueOwned::i64(1), ValueOwned::i64(2)]),
                "custom" = custom,
                "custom_ref" = &custom,
                "custom_copy" = CustomCopy(42),
                "opt_custom_copy" = Some(CustomCopy(43)),
                "opt_string" = opt_string,
                "opt_val_owned" = opt_val_owned,
                "opt_custom" = opt_custom,
                "opt_u16" = opt_u16,
                "opt_str_ref" = opt_str_ref,
                "none_str" = none_str,
                "none_ref" = none_ref,
                "static_str" = static_str,
                "static_val" = static_val,
                "range_len" = RangeVal(1..4),
                "debug_err" = Value::debug(&err),
                "display_err" = Value::display(&err),
            ],
            "preserved {owned_str}",
        );

        // Verify none of the non-Copy locals were moved and mutable stack locals were released.
        static_str = "reassigned";
        assert_eq!(static_str, "reassigned");
        assert_eq!(owned_str, "owned");
        assert_eq!(cow_str, "cow");
        assert_eq!(bytes_vec, [1, 2, 3]);
        assert_eq!(val_owned.view().to_str(), Some("value_owned"));
        assert_eq!(custom.0, "custom");
        assert_eq!(opt_string.as_deref(), Some("opt_str"));
        assert_eq!(opt_val_owned.as_ref().unwrap().view().to_i64(), Some(99));
        assert_eq!(opt_custom.as_ref().unwrap().0, "opt_custom");

        let captured = collector.records.lock().unwrap();
        captured[0].owned.with(|r| {
            let kvs = r.key_values();
            assert_eq!(kvs.get("int_lit").unwrap().to_i64(), Some(8080));
            assert_eq!(kvs.get("float_lit").unwrap().to_f64(), Some(0.5));
            assert_eq!(kvs.get("owned_str").unwrap().to_str(), Some("owned"));
            assert_eq!(kvs.get("opt_string").unwrap().to_str(), Some("opt_str"));
            assert_eq!(kvs.get("opt_u16").unwrap().to_u64(), Some(7));
            assert_eq!(kvs.get("opt_str_ref").unwrap().to_str(), Some("opt_ref"));
            assert_eq!(kvs.get("custom_copy").unwrap().to_u64(), Some(42));
            assert_eq!(kvs.get("opt_custom_copy").unwrap().to_u64(), Some(43));
            assert_eq!(kvs.get("range_len").unwrap().to_u64(), Some(3));
            assert_eq!(kvs.get("display_err").unwrap().to_string(), "boom");
            assert!(kvs.get("none_str").unwrap().to_str().is_none());
            assert!(kvs.get("none_ref").unwrap().to_str().is_none());
        });
    }

    #[test]
    fn hygiene_against_blanket_capture_loc_shadowing_and_level_lock_drop() {
        trait Hijack: Sized {
            fn capture(self) -> Value<'static> {
                Value::none()
            }
        }
        impl<T> Hijack for T {}

        const __LOC: u32 = 7;
        const LOC: u32 = 9;

        let collector = Collector::default();
        let logger = builder().dispatch(|d| d.append(collector.clone())).build();
        let mutex_level = Mutex::new(Level::Info);

        // Verify Hijack::capture is actually in scope (suppressing dead_code) without hijacking
        // macro KV capture, level MutexGuard drops before format args, and __LOC/LOC do not shadow.
        assert!(matches!(
            123_i32.capture().view(),
            crate::kv::ValueView::None
        ));
        crate::log!(
            logger,
            *mutex_level.lock().unwrap(),
            ["state" = "healthy", "port" = 8080],
            "{__LOC} {LOC} unlocked={}",
            mutex_level.try_lock().is_ok(),
        );
        no_prelude::emit(&logger);

        let captured = collector.records.lock().unwrap();
        assert_eq!(captured.len(), 3);
        captured[0].owned.with(|r| {
            assert_eq!(r.payload().to_string(), "7 9 unlocked=true");
            assert_eq!(
                r.key_values().get("state").unwrap().to_str(),
                Some("healthy")
            );
            assert_eq!(r.key_values().get("port").unwrap().to_i64(), Some(8080));
        });
        captured[2].owned.with(|r| {
            assert_eq!(r.payload().to_string(), "no prelude kv");
            assert_eq!(r.key_values().get("k").unwrap().to_i64(), Some(1));
            assert_eq!(r.key_values().get("s").unwrap().to_str(), Some("ok"));
        });
    }

    #[test]
    fn async_future_remains_send_across_await_points() {
        fn assert_send<F: Future + Send + 'static>(f: F) -> F {
            f
        }

        async fn yield_val<T>(v: T) -> T {
            std::future::ready(v).await
        }

        let fut = assert_send(async move {
            let logger = Logger::noop();
            let mutex_level = Mutex::new(Level::Info);
            let counter = Counter(Cell::new(5));
            let err = io::Error::other("async_err");
            crate::log!(
                yield_val(logger).await,
                *mutex_level.lock().unwrap(),
                [
                    "count" = counter,
                    "step" = yield_val(String::from("one")).await,
                    "err" = Value::display(&err),
                    "next" = yield_val(2_u64).await,
                ],
                "completed {} unlocked={}",
                yield_val("ok").await,
                mutex_level.try_lock().is_ok(),
            );
            counter.0.set(6);
            assert_eq!(counter.0.get(), 6);
        });
        drop(fut);
    }
}
