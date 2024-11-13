# CHANGELOG

All notable changes to this project will be documented in this file.

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
