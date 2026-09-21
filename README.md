# Logforth

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MSRV 1.91][msrv-badge]](https://www.whatrustisit.com)
[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/logforth.svg
[crates-url]: https://crates.io/crates/logforth
[docs-badge]: https://docs.rs/logforth/badge.svg
[msrv-badge]: https://img.shields.io/badge/MSRV-1.91-green?logo=rust
[docs-url]: https://docs.rs/logforth
[license-badge]: https://img.shields.io/crates/l/logforth
[license-url]: LICENSE
[actions-badge]: https://github.com/fast/logforth/workflows/CI/badge.svg
[actions-url]:https://github.com/fast/logforth/actions?query=workflow%3ACI

Logforth is a versatile, extensible, and easy-to-use logging framework for Rust applications. It allows you to configure multiple dispatches, filters, and appenders to customize your logging setup according to your needs.

## Getting Started

Add Logforth to your application:

```shell
cargo add logforth
```

Build a logger and pass it to the native logging macros:

```rust
use logforth::{Level, LevelFilter, append};

fn main() {
    let logger = logforth::builder()
        .dispatch(|d| d
            .filter(LevelFilter::MoreSevereEqual(Level::Info))
            .append(append::Stderr::default()))
        .build();

    logforth::info!(logger, "service started");
    logforth::debug!(logger, "this message is filtered out");
    logger.flush();
}
```

The native API is always available and does not depend on the `log` crate. There is no implicit global logger or global maximum level: each logger's dispatches decide what to emit. An unfiltered dispatch accepts every level; a logger without dispatches emits nothing.

## Events and structured fields

Use `trace!`, `debug!`, `info!`, `warn!`, `error!`, or `fatal!`, or `log!(logger, level, ...)` for any Logforth `Level`. `fatal!` records severity; it does not terminate the process or flush.

```rust
use logforth::kv::Value;

let logger = logforth::builder()
    .dispatch(|d| d.append(logforth::append::Stderr::default()))
    .build();
let queue = String::from("background");
logforth::info!(logger, {
    "queue" => queue,
    "completed" => 3u64,
    "healthy" => true,
    "details" => Value::debug(&[1, 2, 3]),
}, "batch complete");
logforth::info!(logger, { "queue.depth" => 0 });
```

Fields use standard `From` conversions into the existing `kv::Value`: scalars retain their types, and strings and existing values are borrowed. Use `Value::none()` for an absent value. Use `Value::display` or `Value::debug` for text and `Value::list` or `Value::map` for nested values. No logging-specific conversion trait is required.

Macros prefilter before evaluating messages, keys, fields, or conversions. Full-record filters may still reject an event afterward. The target defaults to the calling module, and source module/file/line/column identify the actual call site. Existing `RustLogFilter` module directives apply directly. Custom targets remain available through `RecordBuilder` and the `log` bridge.

Event fields describe one event. Existing `Diagnostic` implementations supply dispatch, thread, task, or trace context. These macros do not introduce logger-bound fields, merging precedence, or a context propagation API. Appenders keep their existing field/context handling. For independent filtering or output policy, build independent loggers; use `Arc<Logger>` to share a pipeline.

See the [native example](examples/src/native.rs) for static context and typed fields. Existing text/JSON layouts, rolling files, async appenders, and OTLP consume the same records. [Appender documentation](https://docs.rs/logforth-append-opentelemetry) covers OTLP configuration.

## Lifecycle and dependency logs

Stop logging producers, call `logger.flush()`, and then tear down resources required by appenders. Flush does not disable the logger or provide a transactional delivery guarantee. Independent loggers must each be flushed.

The optional `bridge-log` feature forwards dependency logs into a native logger. Install the bridge once at startup, retaining the `Arc<Logger>` for native calls and shutdown:

```rust
use std::sync::Arc;
use logforth::append;
use logforth::bridge::log::LogBridge;

fn main() -> Result<(), log::SetLoggerError> {
    let logger = Arc::new(logforth::builder()
        .dispatch(|d| d.append(append::Stderr::default()))
        .build());
    log::set_boxed_logger(Box::new(LogBridge::new(logger.clone())))?;
    log::set_max_level(log::LevelFilter::Trace);
    logforth::info!(logger, "native event");
    log::info!("dependency event");
    logger.flush();
    Ok(())
}
```

Add `log` and enable `logforth/bridge-log` for this example. `bridge-log-serde` continues to support Serde fields from the `log` facade. Native Serde capture is outside this API; the existing `Value` model supports explicit nested fields. The `starter-log` helpers remain available for applications using the compatibility facade.

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

Logforth supports filtering log records at the dispatch level. Log records that do not pass the filters will not be sent to the appenders. Logforth provides several built-in filters:

* [`RustLogFilter`] allows you to configure logging levels and targets via the famous `RUST_LOG` directive pattern.

[`RustLogFilter`]: https://docs.rs/logforth-filter-rustlog/*/logforth_filter_rustlog/struct.RustLogFilter.html

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

The optional `log` bridge forwards dependency logs into the same dispatch graph as native events. It preserves the original level, target, source location, and supported structured fields.

## Documentation

Read the online documents at https://docs.rs/logforth.

Components are organized into several crates:

* Core APIs: [`logforth-core`](https://docs.rs/logforth-core)
  * Built-in appenders: `Stdout`, `Stderr`, `Testing`
  * Built-in filters: `LevelFilter`
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
* Filters: `logforth-filter-*`
  * [`logforth-filter-rustlog`](https://docs.rs/logforth-filter-rustlog)
* Diagnostics: `logforth-diagnostic-*`
  * [`logforth-diagnostic-fastrace`](https://docs.rs/logforth-diagnostic-fastrace)
  * [`logforth-diagnostic-task-local`](https://docs.rs/logforth-diagnostic-task-local)
* Bridges: `logforth-bridge-*`
  * [`logforth-bridge-log`](https://docs.rs/logforth-bridge-log)

## Minimum Rust version policy

This crate is built against the latest stable release, and its minimum supported rustc version is 1.91.0.

The policy is that the minimum Rust version required to use this crate can be increased in minor version updates. For example, if Logforth 1.0 requires Rust 1.60.0, then Logforth 1.0.z for all values of z will also require Rust 1.60.0 or newer. However, Logforth 1.y for y > 0 may require a newer minimum version of Rust.

## Maturity

This crate has been in development since 2024-08. It is being used in several production systems stable enough to be considered mature, but it is still evolving.

All modular components are factored out into separate crates. We are working to stabilize the core APIs and several core components, and then release them as `logforth-core` 1.0. Other components will be released as separate crates that depend on `logforth-core`.

## License and Origin

This project is licensed under [Apache License, Version 2.0](LICENSE).

The name `Logforth` comes from an antonym of the [`Logback`](https://logback.qos.ch/) project, and may also be read as a homophone of "log force".
