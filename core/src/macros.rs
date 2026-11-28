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
    use crate::kv::LogValue;
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

    #[inline]
    pub fn capture_val<T: LogValue + ?Sized>(v: &T) -> Value<'_> {
        v.log_value()
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __log_impl {
    (@entry $logger:expr, $level:expr, [$($key:literal = $value:expr),* $(,)?], $($arg:tt)+) => {{
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
                        &[$((
                            $crate::kv::Key::new($key),
                            $crate::__private::capture_val(&($value)),
                        )),*],
                        ::core::format_args!($($arg)+),
                    );
                }
            }
        }
    }};
}

/// Logs a record at the specified [`Level`](crate::record::Level) to a [`Logger`](crate::Logger).
///
/// Supports three call forms:
/// - `log!(logger, level, "format {}", arg)` for a formatted message;
/// - `log!(logger, level, ["key" = value, ...], "format {}", arg)` for structured fields plus a
///   formatted message;
/// - `log!(logger, level, ["key" = value, ...])` for structured fields with an empty message body.
///
/// `$logger` (which may be a [`Logger`](crate::Logger) or any reference/smart pointer dereferencing
/// to [`Logger`](crate::Logger)) and `$level` are evaluated once up front, dropping any temporary
/// guards in `$level` before checking [`Logger::enabled`](crate::Logger::enabled). Structured
/// key-values and format arguments are evaluated at most once and only when `enabled` returns
/// `true`.
///
/// Structured field keys must be string literals (`'static` provenance), and field values borrow in
/// place through [`LogValue`](crate::kv::LogValue) without moving non-`Copy` locals. Because
/// [`Value`](crate::kv::Value) is `Send + Sync`, enclosing `async` futures remain `Send` across
/// `.await` points in macro arguments as long as temporaries held across `.await` are `Send`.
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
