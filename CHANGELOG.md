# Changelog

All significant changes to this project will be documented in this file.

## Unreleased

### Breaking changes

* Raise the minimum supported Rust version (MSRV) from 1.89.0 to 1.91.0.
* Upgrade `jiff` from 0.2 to 1.0 in `logforth-append-file`, `logforth-layout-json`, `logforth-layout-logfmt`, and `logforth-layout-text`.
* Upgrade `opentelemetry`, `opentelemetry-otlp`, and `opentelemetry_sdk` from 0.31.0 to 0.33.1 in `logforth-append-opentelemetry`.
* Remove the `anyhow` dependency from `logforth-core` and redesign `Error`: `Error::new` now accepts `impl Into<Cow<'static, str>>`, `Error::with_source` accepts `impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>` instead of `impl Into<anyhow::Error>` and replaces any existing source instead of panicking, and `Error::{with_context, sources, from_io_error, from_fmt_error}` are removed. Walk `.source()` for causal chains and construct descriptive `Error::new(...).with_source(...)` values at call sites.
* Replace `Trap`, `trap::BestEffortTrap`, `AsyncBuilder::trap`, and `FileBuilder::trap` with `ErrorSink` and `error::{StderrErrorSink, BlackHoleErrorSink}` across `logforth-core`, `logforth`, `logforth-append-async`, and `logforth-append-file`. Configure runtime error handling via `LoggerBuilder::error_sink`, `DispatchBuilder::error_sink`, `AsyncBuilder::error_sink`, or `starter_log` builders instead.
* Replace `&[Box<dyn Diagnostic>]` with `&dyn Diagnostic` across `Filter::{enabled, matches}`, `Layout::format`, `Append::append`, and `MakeBody::create` in `logforth-core`, `logforth`, and peripheral crates, and relax the `'static` supertrait bound on `Diagnostic` to the `Box<dyn Diagnostic>` registration boundary; traverse diagnostics via `diags.visit(&mut visitor)` instead of iterating a slice, and pass `&()` for an empty diagnostic context.
* `LogBridge::new` in `logforth-bridge-log` now accepts `Logger` by value instead of `impl Into<Arc<Logger>>`; pass `logger.clone()` when retaining another handle.
* Remove the `const APPEND: bool` generic parameter from `DispatchBuilder` in `logforth-core`; `DispatchBuilder::append` now returns `Self` in any order, and dispatches configured with no appenders are skipped when registered on `LoggerBuilder`.
* Remove `Copy` from `ThreadLocalDiagnostic` in `logforth-core`, `TaskLocalDiagnostic` in `logforth-diagnostic-task-local`, and `FastraceDiagnostic` in `logforth-diagnostic-fastrace`.
* `FutureExt::with_task_local_context` in `logforth-diagnostic-task-local` now accepts `IntoIterator<Item = (K, V)>` where `K: Into<KeyOwned>` and `V: Into<ValueOwned>` instead of `IntoIterator<Item = (String, String)>`, preserving typed values instead of converting them to strings; pass keys and values directly without `.into()` where type inference would otherwise be ambiguous.
* `File` in `logforth-append-file` now writes directly to target segment files instead of writing to an unnumbered active file and shifting reverse-numbered archives on rollover. Unrotated output remains `<filename>[.<suffix>]`, time-only rotation writes `<filename>.<period>[.<suffix>]`, and size rotation numbers segments in ascending order from `1` (oldest) as `<filename>[.<period>].<seq>[.<suffix>]`; `max_log_files` counts the active file toward the retention limit, and files created under the previous naming scheme are ignored or ordered by ascending sequence number until moved or removed.
* `FileBuilder::rollover_size` in `logforth-append-file` now takes `NonZeroU64` instead of `NonZeroUsize` and rotates before a write would exceed the size limit rather than after; update callers passing `NonZeroUsize` to pass `NonZeroU64`.
* `FileBuilder::build` in `logforth-append-file` now returns `Err` when `filename` or `filename_suffix` is empty, `.` or `..`, or contains path separators or NUL bytes.

### New features

* Add `Error::message` in `logforth-core` to borrow the error's own message string.
* Make `Logger` a pointer-sized `Clone` and `Default` handle over a shared pipeline and add `const fn Logger::noop()` in `logforth-core`, and implement `Clone` for `LogBridge` in `logforth-bridge-log`.
* Add `LoggerBuilder::error_sink`, `DispatchBuilder::error_sink`, and `starter_log::{LogStarterBuilder, LogStarterTestingBuilder, LogStarterStdStreamBuilder}::error_sink` to configure runtime error handling.
* Implement `Append`, `ErrorSink`, `Filter`, and `Layout` in `logforth-core`, and `MakeBody` in `logforth-append-opentelemetry`, for `Arc<T>` and `&'static T` where `T: ?Sized`, and implement `Diagnostic` for `&T`, `Arc<T>`, `()`, `Option<T>`, slices, arrays, and `Vec`.
* Add `FileBuilder::timezone` in `logforth-append-file` (and re-export `jiff`) to configure the timezone used for rotation boundaries and segment filenames, defaulting to the system timezone.

### Improvements

* Implement `Copy` for `kv::{Key, KeyView, Value, ValueView}`, `From<&'static str>` and `From<String>` for `kv::KeyOwned`, `From` scalar and string conversions for `kv::ValueOwned`, and `kv::Visitor` for `FnMut(KeyView, ValueView) -> Result<(), Error>` closures in `logforth-core`.
* Make `LevelFilter::test` a `const fn` in `logforth-core`.
* Implement `Clone` for `RustLogFilter` and `Directive` in `logforth-filter-rustlog`.

### Bug fixes

* `Logger::log` and `Logger::flush` in `logforth-core` no longer panic when writing internal error reports to standard error fails.
* `File` in `logforth-append-file` now opens segment files in append mode so external truncation does not leave NUL holes, avoids panicking on matching prefix and suffix edge cases or out-of-range timestamps, and avoids truncating rotation timestamps or file sizes on 32-bit targets.
