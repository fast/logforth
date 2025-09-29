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
- **Elegant Layouts**: Format log records using predefined layouts or create your own.
- **Enrichable Diagnostics**: Attach additional context to log records for better debugging and analysis.
- **Custom Components**: Easily implement your own appenders, filters, layouts, and diagnostics by implementing the provided traits.
- **Bridges**: Out-of-the-box integration with the `log` crate for seamless logging.

## Getting Started

Add `log` and `logforth` to your `Cargo.toml`:

```shell
cargo add log
cargo add logforth -F starter-log
```

## Simple Usage

Set up a basic logger that outputs to stdout:

```rust
fn main() {
    logforth::starter_log::stdout().apply();

    log::error!("This is an error message.");
    log::info!("This is an info message.");
    log::debug!("This debug message will not be printed by default.");
}
```

By default, all logging except the error level is disabled. You can enable logging at other levels by setting the [`RUST_LOG`](https://docs.rs/logforth-core/*/logforth_core/filter/env_filter/index.html) environment variable. For example, `RUST_LOG=debug cargo run` will print all logs.

## Advanced Usage

Configure multiple dispatches with different filters and appenders:

```rust
use logforth::append;
use logforth::record::LevelFilter;

fn main() {
    logforth::starter_log::builder()
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

Read more demos under the [examples](logforth/examples) directory.

## Documentation

Read the online documents at https://docs.rs/logforth.

Components are organized into several crates:

* Core APIs: [`logforth-core`](https://docs.rs/logforth-core)
  * Built-in appenders: `Stdout`, `Stderr`, `Testing`
  * Built-in filters: `EnvFilter`
  * Built-in layouts: `PlainTextLayout`
  * Built-in diagnostics: `StaticDiagnostic`, `ThreadLocalDiagnostic`
* Appenders: `logforth-append-*`
  * [`logforth-append-fastrace`](https://docs.rs/logforth-append-fastrace)
  * [`logforth-append-file`](https://docs.rs/logforth-append-file)
  * [`logforth-append-journald`](https://docs.rs/logforth-append-journald)
  * [`logforth-append-opentelemetry`](https://docs.rs/logforth-append-opentelemetry)
  * [`logforth-append-syslog`](https://docs.rs/logforth-append-syslog)
* Layouts: `logforth-layout-*`
  * [`logforth-layout-google-cloud-logging`](https://docs.rs/logforth-layout-google-cloud-logging)
  * [`logforth-layout-json`](https://docs.rs/logforth-layout-json)
  * [`logforth-layout-logfmt`](https://docs.rs/logforth-layout-logfmt)
  * [`logforth-layout-text`](https://docs.rs/logforth-layout-text)
* Diagnostics: `logforth-diagnostic-*`
  * [`logforth-diagnostic-fastrace`](https://docs.rs/logforth-diagnostic-fastrace)
* Bridges: `logforth-bridge-*`
  * [`logforth-bridge-log`](https://docs.rs/logforth-bridge-log)

## Minimum Rust version policy

This crate is built against the latest stable release, and its minimum supported rustc version is 1.85.0.

The policy is that the minimum Rust version required to use this crate can be increased in minor version updates. For example, if Logforth 1.0 requires Rust 1.60.0, then Logforth 1.0.z for all values of z will also require Rust 1.60.0 or newer. However, Logforth 1.y for y > 0 may require a newer minimum version of Rust.

## Maturity

This crates has been in development since 2024-08. It is being used in several production systems stable enough to be considered mature, but it is still evolving.

All modular components are factored out into separate crates. We are undergoing to stabilize the core APIs and several core components, and then release them as `logforth-core` 1.0. Other components will be released as separate crates that depend on `logforth-core`.

### Stabilize targets

Fundamental logging APIs are stabilized, including:

* Traits: `Append`, `Layout`, `Filter`, `Diagnostic` and its `Visitor`
* Facades: `DispatchBuilder`, `LoggerBuilder`, and `Logger`

Core appenders, filters, layouts, and diagnostics are also stabilized, including:

* Appenders: `Stdout`, `Stderr`, and `Testing`
* Filters: `EnvFilter`
* Layouts: `TextLayout` and `JsonLayout`
* Diagnostics: `StaticDiagnostic` and `ThreadLocalDiagnostic`

Other appenders, filters, layouts, and diagnostics are still evolving and may change in future versions.

The following components yet to be unstabilized have known production usage and are considered reliable:

* Appenders: `Fastrace`, `OpentelemetryLog`, and `RollingFile`
* Layouts: `LogfmtLayout` and `GoogleCloudLoggingLayout`
* Diagnostics: `FastraceDiagnostic`

## License and Origin

This project is licensed under [Apache License, Version 2.0](LICENSE).

The name `Logforth` comes from an antonym to the [`Logback`](https://logback.qos.ch/) project, and may also be read as a homophone of "log force".
