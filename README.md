# Logforth

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MSRV 1.80][msrv-badge]](https://www.whatrustisit.com)
[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/logforth.svg
[crates-url]: https://crates.io/crates/logforth
[docs-badge]: https://docs.rs/logforth/badge.svg
[msrv-badge]: https://img.shields.io/badge/MSRV-1.80-green?logo=rust
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
cargo add log
cargo add logforth
```

## Simple Usage

Set up a basic logger that outputs to stdout:

```rust
fn main() {
    logforth::stdout().apply();

    log::error!("This is an error message.");
    log::info!("This is an info message.");
    log::debug!("This debug message will not be printed by default.");
}
```

By default, all logging except the error level is disabled. You can enable logging at other levels by setting the [`RUST_LOG`](https://docs.rs/env_logger/*/env_logger/#enabling-logging) environment variable. For example, `RUST_LOG=debug cargo run` will print all logs.

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

This crate is built against the latest stable release, and its minimum supported rustc version is 1.85.0.

The policy is that the minimum Rust version required to use this crate can be increased in minor version updates. For example, if Logforth 1.0 requires Rust 1.60.0, then Logforth 1.0.z for all values of z will also require Rust 1.60.0 or newer. However, Logforth 1.y for y > 0 may require a newer minimum version of Rust.

## Maturity

This crates has been in development since 2024-08. It is being used in several production systems stable enough to be considered mature, but it is still evolving. Read the following sections for what is stabilized and what is planned for the future.

### Stabilize targets

Fundamental logging APIs are stabilized, including:

* Traits: `Append`, `Layout`, `Filter`, `Diagnostic` and its `Visitor`
* Facades: `DispatchBuilder`, `LoggerBuilder`, and `Logger`

Core appenders, filters, layouts, and diagnostics are also stabilized, including:

* Appenders: `Stdout`, `Stderr`, and `Testing`
* Filters: `EnvFilter`
* Layouts: `TestLayout` and `JsonLayout`
* Diagnostics: `StaticDiagnostic` and `ThreadLocalDiagnostic`

Other appenders, filters, layouts, and diagnostics are still evolving and may change in future versions.

The following components yet to be unstabilized have known production usage and are considered reliable:

* Appenders: `Fastrace`, `OpentelemetryLog`, and `RollingFile`
* Layouts: `LogfmtLayout` and `GoogleCloudLoggingLayout`
* Diagnostics: `FastraceDiagnostic`

### Future plans

**What about a 1.0 release?**

The fundamental APIs and core components are stable. It's possible to factor out a separate `logforth-api` (or `logforth-core`) crate that contains only the stable APIs, and then release `logforth-api` 1.0 with the stable APIs and core components. I just don't decide its name and project layout yet.

The rest components, due to their external dependencies and several missing features, are still evolving and may change in future versions. They will be released as `logforth-append-foo`, `logforth-filter-bar`, `logforth-layout-baz`, and `logforth-diagnostic-qux` crates, which will depend on the stable `logforth-api` crate.

**What are the missing features?**

Before stabilize `RollingFile` and `Syslog` appenders that depend on the `NonBlocking` utility, I need to decide whether an `AsyncAppend` composition is better (see [#145](https://github.com/fast/logforth/issues/145)).

Otherwise, how to share utilities like `NonBlocking` and `LevelColor` between different separate crates without duplicating code is still an open question.

**What about components that have external dependencies?**

Fastrace's appenders and diagnostic, OpenTelemetry's appender, etc. have external dependencies that are not stable yet. The best option should be to release them as separate crates, such as `logforth-append-fastrace`, `logforth-diagnostic-fastrace`, `logforth-append-opentelemetry`, etc. This way, they can evolve independently and be used in projects that require them without affecting the core logging functionality.

This is blocked by not having a stable `logforth-api` crate yet, as these components depend on the stable APIs.

**What is the future of the `logforth` crate?**

It will continue to be the main crate that assembles all the features and provides a one-for-all dependency.

## License and Origin

This project is licensed under [Apache License, Version 2.0](LICENSE).

The name `Logforth` comes from an antonym to the [`Logback`](https://logback.qos.ch/) project, and may also be read as a homophone of "log force".
