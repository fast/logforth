# CHANGELOG

All notable changes to this project will be documented in this file.

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
    logforth::dispatch(|b| b.filter(LevelFilter::Debug).append(append::Stderr::default().with_layout(JsonLayout::default())))
        .and_dispatch(|b| b.filter(LevelFilter::Info).append(append::Stdout::default().with_layout(JsonLayout::default())))
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

2. refactor: unify level/target filter to directive filter ([#65](https://github.com/fast/logforth/pull/65))

    Most `From` conversions are kept so that typically you won't notice the change. But if you directly use `LevelFilter` and `TargetFilter`, they are now removed. The functionalities can be covered by `EnvFilter`.

    Also, the feature flag `env-filter` is removed. The `EnvFilter` is always available now.
