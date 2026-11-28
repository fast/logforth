# Logforth Task-Local Diagnostic

`logforth-diagnostic-task-local` provides [`TaskLocalDiagnostic`](https://docs.rs/logforth-diagnostic-task-local/latest/logforth_diagnostic_task_local/struct.TaskLocalDiagnostic.html), [`FutureExt`](https://docs.rs/logforth-diagnostic-task-local/latest/logforth_diagnostic_task_local/trait.FutureExt.html), and [`TaskLocalFuture`](https://docs.rs/logforth-diagnostic-task-local/latest/logforth_diagnostic_task_local/struct.TaskLocalFuture.html) for attaching runtime-agnostic task-local key-value context to `Future`s in Logforth pipelines.

Most applications can enable this crate through the [`logforth`](https://crates.io/crates/logforth) facade crate with the `diagnostic-task-local` feature. Depend on `logforth-diagnostic-task-local` directly when composing split `logforth-*` crates onto `logforth-core`.

## Quick start

```shell
cargo add logforth-core logforth-diagnostic-task-local
```

```rust
use logforth_core::LoggerBuilder;
use logforth_core::append::Stderr;
use logforth_diagnostic_task_local::FutureExt;
use logforth_diagnostic_task_local::TaskLocalDiagnostic;

async fn handle_request(logger: &logforth_core::Logger) {
    logforth_core::info!(logger, "handling request");
}

async fn run() {
    let logger = LoggerBuilder::new()
        .dispatch(|d| {
            d.diagnostic(TaskLocalDiagnostic::new())
                .append(Stderr::new())
        })
        .build();

    handle_request(&logger)
        .with_task_local_context([("request_id", "req-42")])
        .await;
}
```

## Behavior and guarantees

- **Runtime-agnostic scoping**: `FutureExt::with_task_local_context` wraps any `core::future::Future` in a [`TaskLocalFuture`](https://docs.rs/logforth-diagnostic-task-local/latest/logforth_diagnostic_task_local/struct.TaskLocalFuture.html) that pushes its entries while being polled and pops them whenever `poll` yields or finishes, isolating interleaved tasks across `tokio`, `smol`, `futures`, or custom executors.
- **Typed values and shadowing**: Entries are stored as `(KeyOwned, ValueOwned)` without stringifying scalars. Nested `with_task_local_context` scopes shadow outer entries for the same key and restore them when the inner future finishes; duplicate keys within a single scope resolve with last-entry-wins precedence.
- **Teardown safety**: Thread-local storage is accessed via `LocalKey::try_with`, so polling or logging during thread-local destruction degrades to an empty context instead of panicking.

## Documentation

- [Rust API documentation](https://docs.rs/logforth-diagnostic-task-local)
- [Logforth facade crate](https://docs.rs/logforth)

## License

Licensed under the Apache License, Version 2.0.
