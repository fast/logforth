# Logforth Project

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MSRV 1.71][msrv-badge]](https://www.whatrustisit.com)
[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/logforth.svg
[crates-url]: https://crates.io/crates/logforth
[docs-badge]: https://docs.rs/logforth/badge.svg
[msrv-badge]: https://img.shields.io/badge/MSRV-1.71-green?logo=rust
[docs-url]: https://docs.rs/logforth
[license-badge]: https://img.shields.io/crates/l/logforth
[license-url]: LICENSE
[actions-badge]: https://github.com/cratesland/logforth/workflows/CI/badge.svg
[actions-url]:https://github.com/cratesland/logforth/actions?query=workflow%3ACI

## Overview

A versatile and extensible logging implementation.

## Usage

Add the dependencies to your `Cargo.toml` with:

```shell
cargo add log
cargo add logforth
```

... where [log](https://crates.io/crates/log) is the logging facade and [logforth](https://crates.io/crates/logforth) is the logging implementation.

Then, you can use the logger with:

```rust
use log::LevelFilter;
use logforth::append;
use logforth::layout::TextLayout;
use logforth::Dispatch;
use logforth::Logger;

fn main() {
    Logger::new().dispatch(
        Dispatch::new()
            .filter(LevelFilter::Trace)
            .layout(TextLayout::default())
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

Read more demos under the [examples](examples) directory.

## Documentation

Read the online documents at https://docs.rs/logforth.

## Supported Rust Versions (MSRV 1.71)

Logforth is built against the latest stable release. The minimum supported version is 1.71. The current Logforth version is not guaranteed to build on Rust versions earlier than the minimum supported version.

## When to release a 1.0 version

After one year of practicing the interfaces, if there are no further blockers, I'll release a 1.0 version. So consequently, it can be as early as 2025-08.

## License and Origin

This project is licensed under [Apache License, Version 2.0](LICENSE).

The name `Logforth` comes from an antonym to the [`Logback`](https://logback.qos.ch/) project.
