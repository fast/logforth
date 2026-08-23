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

/// Log a message at a dynamically selected level.
///
/// A logger instance is required as the first argument. The target is the caller's module path.
/// Structured key-value pairs precede the message and are separated from it by a semicolon. Values
/// retain their native type by default; use `:?` or `:%` to capture a value with
/// [`Debug`](std::fmt::Debug) or [`Display`](std::fmt::Display).
///
/// Keys can be identifiers, string literals, or parenthesized string expressions. An identifier
/// without `= value` captures the variable with the same name. The message can be omitted for a
/// structured-only record by ending the fields with a semicolon.
///
/// The message and structured fields are not evaluated when the logger disables the level and
/// target.
///
/// # Examples
///
/// ```
/// use logforth_core::record::Level;
///
/// let logger = logforth_core::builder().build();
/// let request_id = 42_u64;
/// logforth_core::log!(
///     logger,
///     Level::Info2,
///     request_id,
///     peer:% = "127.0.0.1";
///     "request accepted"
/// );
/// ```
#[macro_export]
#[clippy::format_args]
macro_rules! log {
    ($logger:expr, $level:expr, $($args:tt)+) => {{
        $crate::__log!($logger, $level, $($args)+)
    }};
    ($($args:tt)*) => {{
        ::std::compile_error!("Logforth logging macros require a logger as the first argument")
    }};
}

/// Log a message at the fatal level.
///
/// This macro records severity only; it does not terminate the process or flush the logger.
///
/// # Examples
///
/// ```
/// let logger = logforth_core::builder().build();
/// logforth_core::fatal!(logger, "unrecoverable failure");
/// ```
#[macro_export]
#[clippy::format_args]
macro_rules! fatal {
    ($logger:expr, $($args:tt)+) => {{
        $crate::log!($logger, $crate::record::Level::Fatal, $($args)+)
    }};
    ($($args:tt)*) => {{
        ::std::compile_error!("Logforth logging macros require a logger as the first argument")
    }};
}

/// Log a message at the error level.
///
/// # Examples
///
/// ```
/// let logger = logforth_core::builder().build();
/// logforth_core::error!(logger, "operation failed");
/// ```
#[macro_export]
#[clippy::format_args]
macro_rules! error {
    ($logger:expr, $($args:tt)+) => {{
        $crate::log!($logger, $crate::record::Level::Error, $($args)+)
    }};
    ($($args:tt)*) => {{
        ::std::compile_error!("Logforth logging macros require a logger as the first argument")
    }};
}

/// Log a message at the warn level.
///
/// # Examples
///
/// ```
/// let logger = logforth_core::builder().build();
/// logforth_core::warn!(logger, "retrying operation");
/// ```
#[macro_export]
#[clippy::format_args]
macro_rules! warn {
    ($logger:expr, $($args:tt)+) => {{
        $crate::log!($logger, $crate::record::Level::Warn, $($args)+)
    }};
    ($($args:tt)*) => {{
        ::std::compile_error!("Logforth logging macros require a logger as the first argument")
    }};
}

/// Log a message at the info level.
///
/// # Examples
///
/// ```
/// let logger = logforth_core::builder().build();
/// logforth_core::info!(logger, user_id = 42_u64; "user connected");
/// ```
#[macro_export]
#[clippy::format_args]
macro_rules! info {
    ($logger:expr, $($args:tt)+) => {{
        $crate::log!($logger, $crate::record::Level::Info, $($args)+)
    }};
    ($($args:tt)*) => {{
        ::std::compile_error!("Logforth logging macros require a logger as the first argument")
    }};
}

/// Log a message at the debug level.
///
/// # Examples
///
/// ```
/// let logger = logforth_core::builder().build();
/// logforth_core::debug!(logger, "state updated");
/// ```
#[macro_export]
#[clippy::format_args]
macro_rules! debug {
    ($logger:expr, $($args:tt)+) => {{
        $crate::log!($logger, $crate::record::Level::Debug, $($args)+)
    }};
    ($($args:tt)*) => {{
        ::std::compile_error!("Logforth logging macros require a logger as the first argument")
    }};
}

/// Log a message at the trace level.
///
/// # Examples
///
/// ```
/// let logger = logforth_core::builder().build();
/// logforth_core::trace!(logger, "entered operation");
/// ```
#[macro_export]
#[clippy::format_args]
macro_rules! trace {
    ($logger:expr, $($args:tt)+) => {{
        $crate::log!($logger, $crate::record::Level::Trace, $($args)+)
    }};
    ($($args:tt)*) => {{
        ::std::compile_error!("Logforth logging macros require a logger as the first argument")
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __log {
    ($logger:expr, $level:expr, $($key:tt $(:$capture:tt)? $(= $value:expr)?),+; $($message:tt)+) => {{
        let __logforth_logger: &$crate::Logger = &$logger;
        let __logforth_level = $level;
        let __logforth_target = ::std::module_path!();
        let __logforth_criteria = $crate::record::FilterCriteria::builder()
            .level(__logforth_level)
            .target(__logforth_target)
            .build();
        if __logforth_logger.enabled(&__logforth_criteria) {
            __logforth_logger.log(
                &$crate::record::Record::builder()
                    .level(__logforth_level)
                    .target_static(__logforth_target)
                    .module_path_static(::std::module_path!())
                    .file_static(::std::file!())
                    .line(::std::option::Option::Some(::std::line!()))
                    .column(::std::option::Option::Some(::std::column!()))
                    .payload(::std::format_args!($($message)+))
                    .key_values(&[
                        $((
                            $crate::__log_key!($key),
                            $crate::__log_value!($key $(:$capture)? $(= $value)?),
                        )),+
                    ][..])
                    .build(),
            );
        }
    }};
    ($logger:expr, $level:expr, $($key:tt $(:$capture:tt)? $(= $value:expr)?),+;) => {{
        $crate::__log!($logger, $level, $($key $(:$capture)? $(= $value)?),+; "")
    }};
    ($logger:expr, $level:expr, $($message:tt)+) => {{
        let __logforth_logger: &$crate::Logger = &$logger;
        let __logforth_level = $level;
        let __logforth_target = ::std::module_path!();
        let __logforth_criteria = $crate::record::FilterCriteria::builder()
            .level(__logforth_level)
            .target(__logforth_target)
            .build();
        if __logforth_logger.enabled(&__logforth_criteria) {
            __logforth_logger.log(
                &$crate::record::Record::builder()
                    .level(__logforth_level)
                    .target_static(__logforth_target)
                    .module_path_static(::std::module_path!())
                    .file_static(::std::file!())
                    .line(::std::option::Option::Some(::std::line!()))
                    .column(::std::option::Option::Some(::std::column!()))
                    .payload(::std::format_args!($($message)+))
                    .build(),
            );
        }
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __log_key {
    ($key:ident) => {
        $crate::kv::Key::new(::std::stringify!($key))
    };
    ($key:literal) => {
        $crate::kv::Key::new($key)
    };
    (($key:expr)) => {
        $crate::kv::Key::borrowed($key)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __log_value {
    ($key:tt = $value:expr) => {
        $crate::kv::ToValue::to_value(&$value)
    };
    ($key:tt :? = $value:expr) => {
        $crate::kv::Value::debug(&$value)
    };
    ($key:tt :debug = $value:expr) => {
        $crate::kv::Value::debug(&$value)
    };
    ($key:tt :% = $value:expr) => {
        $crate::kv::Value::display(&$value)
    };
    ($key:tt :display = $value:expr) => {
        $crate::kv::Value::display(&$value)
    };
    ($key:ident) => {
        $crate::kv::ToValue::to_value(&$key)
    };
    ($key:ident :?) => {
        $crate::kv::Value::debug(&$key)
    };
    ($key:ident :debug) => {
        $crate::kv::Value::debug(&$key)
    };
    ($key:ident :%) => {
        $crate::kv::Value::display(&$key)
    };
    ($key:ident :display) => {
        $crate::kv::Value::display(&$key)
    };
}
