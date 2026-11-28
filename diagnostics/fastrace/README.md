# Logforth Fastrace Diagnostic

`logforth-diagnostic-fastrace` provides [`FastraceDiagnostic`](https://docs.rs/logforth-diagnostic-fastrace/latest/logforth_diagnostic_fastrace/struct.FastraceDiagnostic.html), implementing `logforth_core::Diagnostic` by reading the active thread-local [`fastrace`](https://crates.io/crates/fastrace) span context.

Most applications can enable this crate through the [`logforth`](https://crates.io/crates/logforth) facade crate with the `diagnostic-fastrace` feature. Depend on `logforth-diagnostic-fastrace` directly when composing split `logforth-*` crates onto `logforth-core`.

## Quick start

Enable `fastrace`'s `enable` feature in the final application crate so span contexts are collected at runtime (following `fastrace`'s library convention, `logforth-diagnostic-fastrace` itself leaves `fastrace/enable` off so tracing can still be statically disabled by the binary):

```shell
cargo add logforth-core logforth-diagnostic-fastrace
cargo add fastrace --features enable
```

```rust
use fastrace::Span;
use fastrace::collector::SpanContext;
use logforth_core::LoggerBuilder;
use logforth_core::append::Stderr;
use logforth_diagnostic_fastrace::FastraceDiagnostic;

let logger = LoggerBuilder::new()
    .dispatch(|d| {
        d.diagnostic(FastraceDiagnostic::new())
            .append(Stderr::new())
    })
    .build();

let root = Span::root("handle_request", SpanContext::random());
let _guard = root.set_local_parent();
logforth_core::info!(logger, "processing request");
```

## Emitted keys

When a local parent span is active on the current thread (`SpanContext::current_local_parent()` returns `Some`), `FastraceDiagnostic` visits three well-known entries without heap allocation:

| Key          | Value Type | Format / Semantics                                                 |
| :----------- | :--------- | :----------------------------------------------------------------- |
| `"trace_id"` | `str`      | 32-character lowercase zero-padded hexadecimal `TraceId` (`u128`). |
| `"span_id"`  | `str`      | 16-character lowercase zero-padded hexadecimal `SpanId` (`u64`).   |
| `"sampled"`  | `bool`     | Whether the active span context is marked as sampled.              |

When no local parent span is active (or when `fastrace/enable` is disabled), zero entries are visited.

## Documentation

- [Rust API documentation](https://docs.rs/logforth-diagnostic-fastrace)
- [Logforth facade crate](https://docs.rs/logforth)

## License

Licensed under the Apache License, Version 2.0.
