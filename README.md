# Logforth

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

Logforth is a flexible and easy-to-use logging framework for Rust applications. It allows you to configure multiple dispatches, filters, and appenders to customize your logging setup according to your needs.

## Features

- **Multiple Dispatches**: Configure different logging behaviors for different parts of your application.
- **Flexible Filters**: Use built-in or custom filters to control which log records are processed.
- **Various Appenders**: Output logs to stdout, stderr, files, or even send them to OpenTelemetry collectors.
- **Custom Layouts**: Format log records using predefined layouts or create your own.

## Getting Started

Add `log` and `logforth` to your `Cargo.toml`:

```shell
> cargo add log
> cargo add logforth
```

## Simple Usage

Set up a basic logger that outputs to stdout:

```rust
fn main() {
    logforth::stdout().apply();

    log::info!("This is an info message.");
    log::debug!("This debug message will not be printed by default.");
}
```

## Advanced Usage

Configure multiple dispatches with different filters and appenders:

```rust
use logforth::append;
use log::LevelFilter;

fn main() {
    logforth::builder()
        .dispatch(|d| d
            .filter(LevelFilter::Error)
            .append(append::Stderr::default()))
        .dispatch(|d| d
            .filter(LevelFilter::Info)
            .append(append::Stdout::default()))
        .apply();

    log::error!("This error will be logged to stderr.");
    log::info!("This info will be logged to stdout.");
    log::debug!("This debug message will not be logged.");
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
