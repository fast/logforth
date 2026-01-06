# CHANGELOG

All notable changes to this project will be documented in this file.

## Unreleased

### Breaking changes

* Bump minimum supported Rust version (MSRV) to 1.89.0.
* `Append` has no more `exit` method. Users should compose `logforth::core::default_logger().flush()` with their own graceful shutdown logic.
* `Async` appender's `flush` method is now blocking until all buffered logs are flushed by worker threads. Any errors during flushing will be propagated back to the `flush` caller.
* `Record::payload` is now `std::fmt::Arguments` instead of `Cow<'static, str>`.
* `RecordOwned::as_record` has been removed; use `RecordOwned::with` instead. (This is a limitation of Rust as described [here](https://github.com/rust-lang/rust/issues/92698#issuecomment-3311144848).)

## [0.29.1] 2025-11-03

### Bug fixes

* `Record::target_static` should return `&'static str` instead of `&str`.

## [0.29.0] 2025-11-03

### Breaking changes

* Rename `DefaultTrap` to `BestEffortTrap` for better clarity.
* Rename `logforth_core::record::Metadata` to `FilterCriteria`.
* Redesign `LevelFilter` to allow different comparison methods.
* Redesign `Level` as opentelemetry severity numbers.
    * Add `Level::Fatal` variant to represent fatal level logs.
* `LogStarterStdStreamBuilder`'s setter follows builder pattern.
  * `LogStarterStdStreamBuilder::with_layout` is now `layout`.
  * `LogStarterStdStreamBuilder::with_filter` is now `filter`.

### New features

* Add `logforth-diagnostic-task-local` and `TaskLocalDiagnostic` to support task-local key-value context.

### Bug fixes

* Fix `FastraceDiagnostic` does not format `trace_id` and `span_id` as hex strings.

## [0.28.1] 2025-10-06

### Documentation changes

* `doc_auto_cfg` is now `doc_cfg`.

## [0.28.0] 2025-10-06

### Breaking changes

#### Interfaces

* To work with the `log` crate, now it's recommended to add the "starter-log" feature flag and set both:
    ```rust
    fn main() {
        logforth::starter_log::builder().apply();
    }
    ```
* `TextLayout` is now behind `layout-text` feature flag, and colored is always available when the feature is enabled.
* `EnvFilter` is now self-hosted. Some methods may be changed, but the general user experience should retain:
    * `EnvFilter`'s constructors (`from_env`, etc.) are moved to `EnvFilterBuilder`.
* There is no longer `NonBlocking` related logics.

#### File appender

* `SingleFile` appender is removed. You can replace it with `append::File`.
* `RollingFile` is now `File` and is behind `append-file` flag.
* `File` appender now requires `filename` when constructing.
* `File`'s `filename_prefix` is now renamed to mandatory `filename`.
* `File`'s `max_log_files` now takes `NonZeroUsize`.
* `File`'s rollover strategy methods has been changed:
    * `max_file_size` -> `rollover_size` and takes `NonZeroUsize`
    * `rotation` -> `rollover_minutely`, `rollover_hourly`, `rollover_daily`
    * By default, no rollover is performed.
* Rollover filename strategy has been changed:
    ```
    given:
    filename = app
    filename_suffix = log
    max_log_files = 3

    before rollover:
    app.log
    app.1.log
    app.2.log

    after rollover:
    app.log
    app.1.log - old app.log
    app.2.log - old app.1.log
    - old app.2.log deleted
    ```

#### Developments

* All interfaces that return `anyhow::Result` is now using a result over `logforth::Error`.
* Internal log structs are migrated from `log` crate to self-hosted types. This should not affect most users, but if you are customizing appender, layout, filter, and diagnostic, you should replace `log::Record`, `log::Metadata`, or `log::Level`, with `logforth::Record`, `logforth::Metadata`, or `logforth::Level`.
* All components are factored out into their own crates.

#### Minors

* `JsonLayout` now collects diagnostics context into a separate field `diags`.
* Upgrade to opentelemetry 0.31.0.

### Notable changes

* Timestamp format in `TextLayout`, `JsonLayout`, and `LogfmtLayout` is changed from RFC 9557 to RFC 3339 format.
  * That is, from "2025-01-10T15:22:37.868815+08:00[Asia/Shanghai]" to "2025-01-10T15:22:37.868815+08:00".

### New features

* `PlainTextLayout` is added to support plain text format without any extra dependency.
* `Async` appender is added to support async logging with configurable buffer size and worker threads.
* `Trap` trait and a default `DefaultTrap` is added to support handling internal errors.

## [0.27.0] 2025-08-18

### Notable changes

* Bump the Rust edition to 2024.
* Claim stabilize targets for the crate.

## [0.26.2] 2025-07-13

### Bug fixes

* `RollingFile` appender now correctly delete old files on systems where `create_time` is not available ([#142](https://github.com/fast/logforth/pull/142))

## [0.26.1] 2025-06-05

### New features

* Implement `Testing` appender that sends logs to `eprintln!`.

## [0.26.0] 2025-06-04

### Breaking changes

* `OpenTelemetryLogBuild::build` is now infallible.
* Upgrade to opentelemetry 0.30.0.
* Feature flags are renamed with prefix like `append-rolling-file` and `layout-json`.
* `OpenTelemetryLog` appender now accepts `MakeBody` over `Layout`.
* `Filter::matches` and `Filter::enabled` now take an extra `&[Box<dyn Diagnostic>]` argument to retrieve the mapped diagnostics context.

## [0.25.0] 2025-05-15

### Breaking changes

* `OpenTelemetryLogBuild::new` now accepts `opentelemetry_otlp::LogExporter`.
* Bump minimum supported Rust version (MSRV) to 1.85.0.

### Improvements

* `FastraceDiagnostic` now exports `sampled` attribute.

## [0.24.0] 2025-04-09

### Breaking changes

* `Diagnostic` is now a trait. `Visitor`'s method signature is simplified.
* `Append::flush` is now fallible.
* `Diagnostic`'s and `Visitor`'s `visit` methods are fallible.
* `NonBlocking` related types and the feature flag are now private.
* `logforth::Builder` is renamed to `logforth::LoggerBuilder`.
* `LoggerBuilder` has no longer an option to configure the global `max_level`. Check its documentation for more details.
* Constructing `RollingFile` and `Syslog` appender is heavily simplified.

Before:

```rust
fn construct_rolling_file() {
    let rolling_writer = RollingFileWriter::builder()
        .rotation(Rotation::Daily)
        .filename_prefix("app_log")
        .build("logs")
        .unwrap();

    let (non_blocking, _guard) = rolling_file::non_blocking(rolling_writer).finish();

    logforth::builder()
        .dispatch(|d| {
            d.filter(log::LevelFilter::Trace)
                .append(RollingFile::new(non_blocking).with_layout(JsonLayout::default()))
        })
        .apply();
}

fn construct_syslog() {
    let syslog_writer = SyslogWriter::tcp_well_known().unwrap();
    let (non_blocking, _guard) = syslog::non_blocking(syslog_writer).finish();

    logforth::builder()
        .dispatch(|d| {
            d.filter(log::LevelFilter::Trace)
                .append(Syslog::new(non_blocking))
        })
        .apply();
}
```

After:

```rust
fn construct_rolling_file() {
    let (rolling_writer, _guard) = RollingFileBuilder::new("logs")
        .layout(JsonLayout::default())
        .rotation(Rotation::Daily)
        .filename_prefix("app_log")
        .build()
        .unwrap();

    logforth::builder()
        .dispatch(|d| d.filter(log::LevelFilter::Trace).append(rolling_writer))
        .apply();
}

fn construct_syslog() {
    let (append, _guard) = SyslogBuilder::tcp_well_known().unwrap().build();

    logforth::builder()
        .dispatch(|d| d.filter(log::LevelFilter::Trace).append(append))
        .apply();
}
```


### New features

* Add `LogfmtLayout` to support logfmt format.
* Add `GoogleStructuredLogLayout` to support Google structured log format.
* `LoggerBuilder` now has a `build` method to construct the `Logger` for use.

## [0.23.1] 2025-03-23

### Improvements

* Upgrade to opentelemetry 0.29.0 to avoid transitive dependency to axum.

## [0.23.0] 2025-03-17

### Breaking changes

* `Layout` and `Filter` as traits
* Default features are now empty; no more `colored` in default.

### New features

* Add `StaticDiagnostic` for globally configuring context.

## [0.22.1] 2025-02-20

### Refactor

* Revisit all `pub(crate)` methods:
  * Expose `Layout::format` and all layout impls `format` method.
  * Expose `Nonblocking` methods to make it usable outside this crate.

## [0.22.0] 2025-02-13

### Breaking changes

* Upgrade `jiff` to 0.2.0 and `opentelemetry` to 0.28.0.

## [0.21.0] 2025-01-15

### Breaking changes

* Re-export `colored` to decouple the dependency. In addition, replace the `no-color` feature flag with the `colored` feature flag.

## [0.20.0] 2025-01-08

### Breaking changes

* Bump MSRV to 1.80 for upgrading `colored` to 3.0.
* Layout and Appender now accept a new argument `diagnostics: &[Diagnostic]` to retrieve the mapped diagnostics context. Users can use `logforth::diagnostic::Visitor` to visit the diagnostics context.

### New features

* Add `logforth::diagnostic::FastraceDiagnostic` to support attaching trace id to as key-value context.
* Add `logforth::diagnostic::ThreadLocalDiagnostic` to support attaching thread local key-value context.

## [0.19.2] 2025-01-03

### Fixes

* Fix minimum version required for env_filter should be 0.1.1 ([#87](https://github.com/fast/logforth/pull/87)).

## [0.19.1] 2024-12-20

### Refactor

* Migrate from `log`'s `kv_unstable` feature to `kv` feature ([#85](https://github.com/fast/logforth/pull/85)).

## [0.19.0] 2024-12-07

### Breaking Changes

* `module_path` is replaced by `target` in `JsonLayout` and `TextLayout` ([#82](https://github.com/fast/logforth/pull/82)).
* Error perform logging now prints error in Debug format ([#84](https://github.com/fast/logforth/pull/84))

## [0.18.1] 2024-11-17

### New features

* Re-export `broadcast` and `native_tls` constructors from fasyslog ([#81](https://github.com/fast/logforth/pull/81)).

## [0.18.0] 2024-11-14

### Breaking changes

* The mapping between syslog severity and log's level is changed.
  * `log::Level::Error` is mapped to `syslog::Severity::Error` (unchanged).
  * `log::Level::Warn` is mapped to `syslog::Severity::Warning` (unchanged).
  * `log::Level::Info` is mapped to `syslog::Severity::Notice` (changed).
  * `log::Level::Debug` is mapped to `syslog::Severity::Info` (changed).
  * `log::Level::Trace` is mapped to `syslog::Severity::Debug` (unchanged).

### New features

* Add `journald` feature to support journald appenders ([#80](https://github.com/fast/logforth/pull/80)).

## [0.17.1] 2024-11-12

### Refactors

* Change the re-export of `syslog` to `logforth::syslog::fasyslog` ([#74](https://github.com/fast/logforth/pull/74))

## [0.17.0] 2024-11-12

### New features

* Add `syslog` feature to support syslog appenders ([#72](https://github.com/fast/logforth/pull/72))

### Breaking changes

Two breaking changes in [#72](https://github.com/fast/logforth/pull/72):

1. `rolling_file` feature flag is renamed to `rolling-file`.
2. `NonBlocking` related structures and methods are relocated, now you'd construct a non-blocking like:

```rust
fn main() {
    let rolling_writer = RollingFileWriter::builder()
        .rotation(Rotation::Daily)
        .filename_prefix("app_log")
        .build("logs")
        .unwrap();

    let (non_blocking, _guard) = rolling_file::non_blocking(rolling_writer).finish();

    logforth::builder()
        .dispatch(|d| {
            d.filter(log::LevelFilter::Trace)
                .append(RollingFile::new(non_blocking).with_layout(JsonLayout::default()))
        })
        .apply();
}
```

or:

```rust
fn main() {
    let syslog_writer = SyslogWriter::tcp_well_known().unwrap();
    let (non_blocking, _guard) = syslog::non_blocking(syslog_writer).finish();

    logforth::builder()
        .dispatch(|d| {
            d.filter(log::LevelFilter::Trace)
                .append(Syslog::new(non_blocking))
        })
        .apply();
}
```

Note that each `NonBlocking` now has a type parameter to ensure that they match the corresponding appenders.

## [0.16.0] 2024-10-30

### Breaking changes

Two minor breaking changes in [#71](https://github.com/fast/logforth/pull/71):

1. `JsonLayout`'s field `tz` is now private. You can change it with the new `timezone` method, like `JsonLayout::default().timezone(TimeZone::UTC)`.
2. `DispatchBuilder` now always accepts `filter` first, and then `append`. Once an `append` is configured, no more `filter` can be added. This is for a strict order on config so that the code is more consistent.

## [0.15.0] 2024-10-30

### Breaking changes

API is further improve in both [#69](https://github.com/fast/logforth/pull/69) and [#70](https://github.com/fast/logforth/pull/70).

Now the logger build logic is like:

```rust
use log::LevelFilter;
use logforth::append;
use logforth::layout::JsonLayout;

fn main() {
    logforth::builder()
        .dispatch(|b| b.filter(LevelFilter::Debug).append(append::Stderr::default().with_layout(JsonLayout::default())))
        .dispatch(|b| b.filter(LevelFilter::Info).append(append::Stdout::default().with_layout(JsonLayout::default())))
        .apply();
}
```

And we provide a convenient way to build the logger with default setup (stderr or stdout, with RUST_LOG envvar respected):

```rust
fn main() {
    logforth::stderr().apply();
    // or logforth::stdout().apply(); // for logging to stdout
}
```

## [0.14.0] 2024-10-28

### Breaking changes

1. refactor: layouts and encoders should be nested to appenders ([#64](https://github.com/fast/logforth/pull/64))

    Previous code:

    ```rust
    fn main() {
        Logger::new()
            .dispatch(
                Dispatch::new()
                    .filter(LevelFilter::Trace)
                    .layout(JsonLayout::default())
                    .append(append::Stdout),
            )
            .apply()
            .unwrap();

        log::error!("Hello error!");
        log::warn!("Hello warn!");
        log::info!("Hello info!");
        log::debug!("Hello debug!");
        log::trace!("Hello trace!");
    }
    ```

    New code:

    ```rust
    fn main() {
        Logger::new()
            .dispatch(
                Dispatch::new()
                    .filter(LevelFilter::Trace)
                    .append(append::Stdout::default().with_layout(JsonLayout::default())),
            )
            .apply()
            .unwrap();

        log::error!("Hello error!");
        log::warn!("Hello warn!");
        log::info!("Hello info!");
        log::debug!("Hello debug!");
        log::trace!("Hello trace!");
    }
    ```

    Besides, the default layout of Stdout, Stderr, and RollingFile is changed from `IdenticalLayout` to `TextLayout`.

2. refactor: unify level/target filter to directive filter ([#65](https://github.com/fast/logforth/pull/65))

    Most `From` conversions are kept so that typically you won't notice the change. But if you directly use `LevelFilter` and `TargetFilter`, they are now removed. The functionalities can be covered by `EnvFilter`.

    Also, the feature flag `env-filter` is removed. The `EnvFilter` is always available now.
