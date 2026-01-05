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

Logforth is a versatile, extensible, and easy-to-use logging framework for Rust applications. It allows you to configure multiple dispatches, filters, and appenders to customize your logging setup according to your needs.

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

By default, all logging except the error level is disabled. You can enable logging at other levels by setting the [`RUST_LOG`](https://docs.rs/logforth-core/*/logforth_core/filter/env_filter/index.html) environment variable. For example, `RUST_LOG=trace cargo run` will print all logs.

## Advanced Usage

Configure multiple dispatches with different filters and appenders:

```rust
fn main() {
    logforth::starter_log::builder()
        .dispatch(|d| d
            .filter(LevelFilter::MoreSevereEqual(Level::Error))
            .append(append::Stderr::default()))
        .dispatch(|d| d
            .filter(LevelFilter::MoreSevereEqual(Level::Info))
            .append(append::Stdout::default()))
        .apply();

    log::error!("This error will be logged to stderr.");
    log::info!("This info will be logged to stdout.");
    log::debug!("This debug message will not be logged.");
}
```

Configure OpenTelemetry appender to export logs to an OpenTelemetry backend ([full example](https://github.com/scopedb/percas/blob/d01db13b/crates/server/src/telemetry.rs#L131-L227)):

```rust
fn main() {
    let static_diagnostic = {
        let mut static_diagnostic = StaticDiagnostic::default();
        static_diagnostic.insert("node_id", node_id);
        static_diagnostic.insert("nodegroup", nodegroup);
        static_diagnostic
    };

    let runtime = async_runtime();
    let filter = make_rust_log_filter(&opentelemetry.filter);
    let appender = runtime.block_on(async {
        let exporter = opentelemetry_otlp::LogExporter::builder()
            .with_tonic()
            .with_endpoint(&opentelemetry.otlp_endpoint)
            .with_protocol(opentelemetry_otlp::Protocol::Grpc)
            .build()
            .expect("failed to initialize opentelemetry logger");

        append::opentelemetry::OpentelemetryLogBuilder::new(service_name, exporter)
            .label("service.name", service_name)
            .build()
    });

    logforth::starter_log::builder().dispatch(|b| {
        b.filter(filter)
            .diagnostic(FastraceDiagnostic::default())
            .diagnostic(static_diagnostic)
            .append(appender)
    });
}
```

Read more demos under the [examples](examples) directory.

## Features

### Dispatches

Logforth supports multiple dispatches, each with its own set of filters and appenders. This allows you to route log messages to different destinations based on their severity or other criteria.

A simple application may only need one dispatch, while a more complex application can have multiple dispatches for different modules or components.

### Appenders

Logforth supports a wide range of built-in appenders implemented as separate crates.

* [`Stdout`] and [`Stderr`] appenders for console logging.
* [`File`] appender for logging to optionally rolling files.
* [`OpentelemetryLog`] appender for exporting logs to OpenTelemetry backends.
* [`Testing`] appender that writes log records that can be captured by a test harness.
* [`FastraceEvent`] appender that writes log records to the [Fastrace](https://docs.rs/fastrace/*/fastrace/) tracing system.
* [`Async`] combiner appender that makes another appender asynchronous.
* `Syslog` and `Journald` appenders for logging to system log services.

[`Stdout`]: https://docs.rs/logforth/*/logforth/append/struct.Stdout.html
[`Stderr`]: https://docs.rs/logforth/*/logforth/append/struct.Stderr.html
[`File`]: https://docs.rs/logforth/*/logforth/append/struct.File.html
[`OpentelemetryLog`]: https://docs.rs/logforth/*/logforth/append/struct.OpentelemetryLog.html
[`Testing`]: https://docs.rs/logforth/*/logforth/append/struct.Testing.html
[`FastraceEvent`]: https://docs.rs/logforth/*/logforth/append/struct.FastraceEvent.html
[`Async`]: https://docs.rs/logforth/*/logforth/append/struct.Async.html

Users can also create their own appenders by implementing the [`Append`] trait.

[`Append`]: https://docs.rs/logforth-core/*/logforth_core/append/trait.Append.html

### Layouts

Some appenders support customizable layouts for formatting log records. Logforth provides several built-in layouts:

* [`TextLayout`] formats log records as optionally colored text.
* [`JsonLayout`] formats log records as JSON objects.
* [`LogfmtLayout`] formats log records in the logfmt style.
* [`GoogleCloudLoggingLayout`] formats log records for Google Cloud Logging.

[`TextLayout`]: https://docs.rs/logforth/*/logforth/layout/struct.TextLayout.html
[`JsonLayout`]: https://docs.rs/logforth/*/logforth/layout/struct.JsonLayout.html
[`LogfmtLayout`]: https://docs.rs/logforth/*/logforth/layout/struct.LogfmtLayout.html
[`GoogleCloudLoggingLayout`]: https://docs.rs/logforth/*/logforth/layout/struct.GoogleCloudLoggingLayout.html

Users can also create their own layouts by implementing the [`Layout`] trait.

[`Layout`]: https://docs.rs/logforth-core/*/logforth_core/layout/trait.Layout.html

The following appenders do *not* use layouts:

* `Async` appender simply forwards log records to another appender; layouts are determined by the inner appender.
* `FastraceEvent` appender converts log records into tracing events; layouts are not applicable.
* `OpentelemetryLog` appender uses `MakeBody` trait for converting log records into OpenTelemetry log bodies. The `MakeBody` trait is more flexible and may optionally use a `Layout` implementation internally.

### Filters

Logforth provides a built-in [`EnvFilter`] that allows you to configure logging levels and targets via the `RUST_LOG` environment variable.

[`EnvFilter`]: https://docs.rs/logforth/*/logforth/filter/struct.EnvFilter.html

Users can also create their own filters by implementing the [`Filter`] trait.

[`Filter`]: https://docs.rs/logforth-core/*/logforth_core/filter/trait.Filter.html

### Diagnostics

Logforth supports providing a mapped diagnostic context (MDC) for stamping each log request.

* [`StaticDiagnostic`] allows you to set static key-value pairs for the entire application, like application version or hostname.
* [`ThreadLocalDiagnostic`] allows you to set thread-local key-value pairs.
* [`TaskLocalDiagnostic`] allows you to set task-local key-value pairs for async tasks.
* [`FastraceDiagnostic`] integrates with the [Fastrace](https://docs.rs/fastrace/*/fastrace/) tracing system to provide tracing context (TraceId, SpanId, etc.) as diagnostics.

[`StaticDiagnostic`]: https://docs.rs/logforth/*/logforth/diagnostic/struct.StaticDiagnostic.html
[`ThreadLocalDiagnostic`]: https://docs.rs/logforth/*/logforth/diagnostic/struct.ThreadLocalDiagnostic.html
[`TaskLocalDiagnostic`]: https://docs.rs/logforth/*/logforth/diagnostic/struct.TaskLocalDiagnostic.html
[`FastraceDiagnostic`]: https://docs.rs/logforth/*/logforth/diagnostic/struct.FastraceDiagnostic.html

Users can also provide their own MDC by implementing the [`Diagnostic`] trait.

[`Diagnostic`]: https://docs.rs/logforth-core/*/logforth_core/diagnostic/trait.Diagnostic.html

### Bridges

So far, Logforth provides out-of-the-box integration with the `log` crate. You can use Logforth as the backend for any crate that uses the `log` facade.

## Documentation

Read the online documents at https://docs.rs/logforth.

Components are organized into several crates:

* Core APIs: [`logforth-core`](https://docs.rs/logforth-core)
  * Built-in appenders: `Stdout`, `Stderr`, `Testing`
  * Built-in filters: `EnvFilter`
  * Built-in layouts: `PlainTextLayout`
  * Built-in diagnostics: `StaticDiagnostic`, `ThreadLocalDiagnostic`
* Appenders: `logforth-append-*`
  * [`logforth-append-async`](https://docs.rs/logforth-append-async)
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
  * [`logforth-diagnostic-task-local`](https://docs.rs/logforth-diagnostic-task-local)
* Bridges: `logforth-bridge-*`
  * [`logforth-bridge-log`](https://docs.rs/logforth-bridge-log)

## Minimum Rust version policy

This crate is built against the latest stable release, and its minimum supported rustc version is 1.85.0.

The policy is that the minimum Rust version required to use this crate can be increased in minor version updates. For example, if Logforth 1.0 requires Rust 1.60.0, then Logforth 1.0.z for all values of z will also require Rust 1.60.0 or newer. However, Logforth 1.y for y > 0 may require a newer minimum version of Rust.

## Maturity

This crate has been in development since 2024-08. It is being used in several production systems stable enough to be considered mature, but it is still evolving.

All modular components are factored out into separate crates. We are working to stabilize the core APIs and several core components, and then release them as `logforth-core` 1.0. Other components will be released as separate crates that depend on `logforth-core`.

## License and Origin

This project is licensed under [Apache License, Version 2.0](LICENSE).

The name `Logforth` comes from an antonym of the [`Logback`](https://logback.qos.ch/) project, and may also be read as a homophone of "log force".
