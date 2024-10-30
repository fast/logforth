# Logforth Project

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MSRV 1.75][msrv-badge]](https://www.whatrustisit.com)
[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/logforth.svg
[crates-url]: https://crates.io/crates/logforth
[docs-badge]: https://docs.rs/logforth/badge.svg
[msrv-badge]: https://img.shields.io/badge/MSRV-1.75-green?logo=rust
[docs-url]: https://docs.rs/logforth
[license-badge]: https://img.shields.io/crates/l/logforth
[license-url]: LICENSE
[actions-badge]: https://github.com/fast/logforth/workflows/CI/badge.svg
[actions-url]:https://github.com/fast/logforth/actions?query=workflow%3ACI

## Overview

A versatile and extensible logging implementation.

## Usage

Add the dependencies to your `Cargo.toml` with:

```shell
cargo add log
cargo add logforth
```

... where [log](https://crates.io/crates/log) is the logging facade and [logforth](https://crates.io/crates/logforth) is the logging implementation.

Then, you can use the logger with the simplest default setup:

```rust
fn main() {
    logforth::stderr().apply();
}
```

Or configure the logger in a more fine-grained way:

```rust
use log::LevelFilter;
use logforth::append;

fn main() {
    logforth::builder()
        .dispatch(|d| d.filter(LevelFilter::Debug).append(append::Stderr::default()))
        .dispatch(|d| d.filter(LevelFilter::Info).append(append::Stdout::default()))
        .apply();
}
```

Read more demos under the [examples](examples) directory.

## Documentation

Read the online documents at https://docs.rs/logforth.

## Minimum Rust version policy

This crate is built against the latest stable release, and its minimum supported rustc version is 1.75.0.

The policy is that the minimum Rust version required to use this crate can be increased in minor version updates. For example, if Logforth 1.0 requires Rust 1.20.0, then Logforth 1.0.z for all values of z will also require Rust 1.20.0 or newer. However, Logforth 1.y for y > 0 may require a newer minimum version of Rust.

## When to release a 1.0 version

After one year of practicing the interfaces, if there are no further blockers, I'll release a 1.0 version. So consequently, it can be as early as 2025-08.

## License and Origin

This project is licensed under [Apache License, Version 2.0](LICENSE).

The name `Logforth` comes from an antonym to the [`Logback`](https://logback.qos.ch/) project.
