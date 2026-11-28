# Logforth Core

`logforth-core` provides the logging pipeline, structured record and key-value model, native logging macros, operational error handling, and extension traits used by Logforth.

Most applications should depend on the [`logforth`](https://crates.io/crates/logforth) facade crate, which re-exports `logforth-core` and wires optional appender, layout, filter, diagnostic, and ecosystem bridge crates behind feature flags.

Depend on `logforth-core` directly when implementing a custom `Append`, `Layout`, `Filter`, `Diagnostic`, or `ErrorSink`, accepting an injected `Logger` in a library (defaulting to `Logger::noop()`), or composing split `logforth-*` crates without the facade.

## Quick start

`logforth-core` carries zero mandatory third-party dependencies and includes built-in console/test appenders (`Stdout`, `Stderr`, `Testing`), `PlainTextLayout`, `LevelFilter`, `StaticDiagnostic`, `ThreadLocalDiagnostic`, and `StderrErrorSink` / `BlackHoleErrorSink`:

```shell
cargo add logforth-core
```

```rust
use logforth_core::Logger;
use logforth_core::append;
use logforth_core::record::Level;
use logforth_core::record::LevelFilter;

fn build_logger() -> Logger {
    logforth_core::builder()
        .dispatch(|d| {
            d.filter(LevelFilter::MoreSevereEqual(Level::Info))
                .append(append::Stdout::default())
        })
        .build()
}

let logger = build_logger();
logforth_core::info!(logger, ["port" = 8080, "tls" = true], "server listening");
logger.flush();
```

Libraries that accept an optional logger from callers can store a `Logger` directly and default to the zero-allocation `Logger::noop()` handle when logging is disabled.

## Compose split crates

Applications that depend on `logforth-core` directly compose filters, diagnostics, layouts, and appenders from individual `logforth-*` crates:

- A filter crate exports a type implementing `logforth_core::Filter`; pass it to `DispatchBuilder::filter`.
- A diagnostic crate exports a type implementing `logforth_core::Diagnostic`; pass it to `DispatchBuilder::diagnostic`.
- A layout crate exports a type implementing `logforth_core::Layout`; pass it to byte-stream appenders such as `Stdout::with_layout` or `FileBuilder::layout`.
- An appender crate exports a type implementing `logforth_core::Append`; pass it to `DispatchBuilder::append`.

```shell
cargo add logforth-core logforth-filter-rustlog logforth-layout-json
```

```rust,ignore
use logforth_core::Logger;
use logforth_core::append::Stderr;
use logforth_filter_rustlog::RustLogFilterBuilder;
use logforth_layout_json::JsonLayout;

fn build_json_logger() -> Result<Logger, logforth_core::Error> {
    let filter = RustLogFilterBuilder::from_default_env_or("info")?.build();
    let appender = Stderr::default().with_layout(JsonLayout::default());

    Ok(logforth_core::builder()
        .dispatch(|d| d.filter(filter).append(appender))
        .build())
}
```

## Feature flags

`logforth-core` enables no features by default:

- `serde`: implements `serde::Serialize` for key-value types and adds `kv::ValueOwned::from_serde` and `kv::ValueOwned::try_from_serde` for capturing arbitrary `serde::Serialize` values as structured `ValueOwned` trees.

## Documentation

- [Rust API documentation](https://docs.rs/logforth-core)
- [Logforth facade crate](https://docs.rs/logforth)

## License

Licensed under the Apache License, Version 2.0.
