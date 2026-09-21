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

/// Emit a native log event through an explicit [`Logger`](crate::Logger).
///
/// The logger is borrowed and evaluated once. An optional field map follows the
/// logger (and level for `log!`). Keys are string expressions;
/// values implement [`ToValue`](crate::kv::ToValue). A fields-only event has an
/// empty message. The message uses the standard Rust formatting syntax.
///
/// Logger and level are evaluated once before metadata filtering. Message, key,
/// and value expressions are only evaluated if at least one dispatch may accept
/// the event. Full-record filters can still reject it. Formatting can happen more
/// than once when several appenders consume the same event.
///
/// A named logger uses its name as the target; otherwise the target is the calling
/// module. Module, file, line, and column always describe the actual call site. All native levels
/// are supported, including `Level::Fatal`, which neither terminates the process nor implicitly
/// flushes the logger.
///
/// ```
/// use logforth_core::{builder, info, log};
/// use logforth_core::kv::Value;
/// use logforth_core::record::Level;
///
/// let logger = builder().build();
/// let count = 3;
/// info!(logger, "processed {count} jobs");
/// log!(logger.named("worker"), Level::Info2, {
///     "jobs.completed" => count,
///     "healthy" => true,
///     "details" => Value::debug(&[1, 2, 3]),
/// }, "batch complete");
/// info!(logger, { "queue.depth" => 0 });
/// logger.flush();
/// ```
#[macro_export]
macro_rules! log {
    ($logger:expr, $level:expr, { $($key:expr => $value:expr),* $(,)? } $(, $($message:tt)*)?) => {
        $crate::__log!($logger, $level, { $($key => $value),* }, $($($message)*)?)
    };
    ($logger:expr, $level:expr, $($message:tt)+) => {
        $crate::__log!($logger, $level, {}, $($message)+)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __log {
    ($logger:expr, $level:expr, { $($key:expr => $value:expr),* }, $($message:tt)+) => {{
        // The match keeps temporary logger expressions alive through dispatch.
        match (&$logger, $level) {
            (logger, level) => {
                let logger: &$crate::Logger = logger;
                let target = logger.name().unwrap_or(::core::module_path!());
                let criteria = $crate::record::FilterCriteria::builder()
                    .level(level)
                    .target(target)
                    .build();
                if logger.enabled(&criteria) {
                    logger.log(&$crate::record::Record::builder()
                        .level(level)
                        .target_static(target)
                        .module_path_static(::core::module_path!())
                        .file_static(::core::file!())
                        .line(::core::option::Option::Some(::core::line!()))
                        .column(::core::option::Option::Some(::core::column!()))
                        .payload(::core::format_args!($($message)+))
                        .key_values(&[
                            $(($crate::kv::Key::borrowed(&$key), $crate::kv::ToValue::to_value(&$value))),*
                        ][..] as &[($crate::kv::Key<'_>, $crate::kv::Value<'_>)])
                        .build());
                }
            }
        }
    }};
    ($logger:expr, $level:expr, { $($key:expr => $value:expr),* },) => {
        $crate::__log!($logger, $level, { $($key => $value),* }, "")
    };
}

/// Emit a [`Trace`](crate::record::Level::Trace) event. See [`log!`](crate::log!) for syntax.
#[macro_export]
macro_rules! trace {
    ($logger:expr, $($event:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Trace, $($event)+)
    };
}

/// Emit a [`Debug`](crate::record::Level::Debug) event. See [`log!`](crate::log!) for syntax.
#[macro_export]
macro_rules! debug {
    ($logger:expr, $($event:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Debug, $($event)+)
    };
}

/// Emit a [`Info`](crate::record::Level::Info) event. See [`log!`](crate::log!) for syntax.
#[macro_export]
macro_rules! info {
    ($logger:expr, $($event:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Info, $($event)+)
    };
}

/// Emit a [`Warn`](crate::record::Level::Warn) event. See [`log!`](crate::log!) for syntax.
#[macro_export]
macro_rules! warn {
    ($logger:expr, $($event:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Warn, $($event)+)
    };
}

/// Emit a [`Error`](crate::record::Level::Error) event. See [`log!`](crate::log!) for syntax.
#[macro_export]
macro_rules! error {
    ($logger:expr, $($event:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Error, $($event)+)
    };
}

/// Emit a [`Fatal`](crate::record::Level::Fatal) event. See [`log!`](crate::log!) for syntax.
/// This records severity only; it does not terminate the process or flush.
#[macro_export]
macro_rules! fatal {
    ($logger:expr, $($event:tt)+) => {
        $crate::log!($logger, $crate::record::Level::Fatal, $($event)+)
    };
}
