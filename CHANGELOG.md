# CHANGELOG

All notable changes to this project will be documented in this file.

## [0.14.0] 2024-10-28

Breaking changes:

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
