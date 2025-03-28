# CHANGELOG

All notable changes to this project will be documented in this file.

## Unreleased

### Breaking changes

* `Diagnosic` is now a trait. `Visitor`'s method signature is simplified.

### New features

* Add `BlockingRollingFile` and `BlockingSyslog` appenders. They would lock the inner appender and log synchronously. In some lower rate logging scenarios, this would be more efficient than the non-blocking version.

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
