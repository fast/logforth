# Logforth Project

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/logforth.svg
[crates-url]: https://crates.io/crates/logforth
[docs-badge]: https://docs.rs/logforth/badge.svg
[docs-url]: https://docs.rs/logforth
[license-badge]: https://img.shields.io/crates/l/logforth
[license-url]: LICENSE
[actions-badge]: https://github.com/tisonkun/logforth/workflows/CI/badge.svg
[actions-url]:https://github.com/tisonkun/logforth/actions?query=workflow%3ACI

## Overview

A versatile and extensible logging implementation.

## Usage

Add the dependency to your `Cargo.toml` with:

```shell
cargo add logforth
```

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

## Supported Rust Versions (MSRV 1.71)

Logforth is built against the latest stable release. The minimum supported version is 1.71. The current Logforth version is not guaranteed to build on Rust versions earlier than the minimum supported version.

## License and Origin

This project is licensed under [Apache License, Version 2.0](LICENSE).

The name `Logforth` comes from an antonym to the [`Logback`](https://logback.qos.ch/) project.
