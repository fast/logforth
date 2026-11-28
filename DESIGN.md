# Architectural Design Guidelines

This document states the workspace-wide architectural direction, pipeline invariants, and cross-crate conventions for Logforth. Concrete type signatures, crate-specific behavior, and runnable examples live in each crate's API documentation (`//!` and `///`) and self-contained `<crate>/README.md` files; repository workflows and style conventions live in [AGENTS.md](AGENTS.md).

## Scope and Crate Versioning

- `logforth-core` carries zero mandatory third-party dependencies so stabilizing `logforth-core 1.0` freezes the core pipeline, structured event model, and extension traits without coupling SemVer stability to external SDKs.
- The `logforth` umbrella facade and peripheral appenders, layouts, filters, diagnostics, and ecosystem bridges evolve on independent `0.x` tracks so external integrations (`opentelemetry`, `jiff`, `fastrace`, `log`, and system daemons) can upgrade at their own pace.

## Pipeline Topology and Composition

Logforth separates three concerns into distinct primitives:

| Dimension            | Question Answered                                         | Representation in Logforth                                    |
| :------------------- | :-------------------------------------------------------- | :------------------------------------------------------------ |
| Source location      | Where in the source tree was the event emitted?           | Compile-time `module_path`, `file`, `line`, and `column`      |
| Channel (`Logger`)   | Which reliability, schema, and lifecycle stream owns it?  | Explicit `Logger` instances                                   |
| Fan-out (`Dispatch`) | Which output slices and destinations receive this stream? | Ordered `Dispatch` branches inside a `Logger`                 |

- `Logger` is a lightweight `Clone` handle over a shared immutable pipeline, with no ambient global singleton in `logforth-core`. Applications expose process-wide channels through explicit statics or dependency injection, while libraries accepting an optional logger default to a zero-allocation `Logger::noop()`.
- All five extension points (`Filter`, `Diagnostic`, `Layout`, `Append`, and `ErrorSink`) are open object-safe traits paired with concrete types rather than closed enums or ad-hoc callbacks, forwarding shared references (`Arc<T>` and `&'static T`) so dispatches and loggers compose as a directed acyclic graph. `Diagnostic` omits a `'static` supertrait bound so decorators can chain stack-borrowed per-call context without allocation, enforcing `'static` only when registering long-lived diagnostics on a `Dispatch`.
- `Append` consumes structured records and diagnostics directly so protocol-native sinks encode typed fields without intermediate text serialization, whereas byte-stream sinks compose a `Layout` internally. `Layout` appends formatted bytes into reusable caller-supplied buffers without modifying prior contents or adding record framing (such as trailing newlines, which belong to the owning `Append`).
- Configuration placement follows pipeline role: `LoggerBuilder` sets the fallback `ErrorSink`, `DispatchBuilder` configures branch-wide filters, diagnostics, appenders, and optional `ErrorSink` overrides as a unit (skipping appender-less branches), and out-of-band executors manage their own worker `ErrorSink`.

## Filtering and Fast-Path Pruning

- Severity levels align 1-to-1 with the 24 OpenTelemetry severity numbers (`1..=24`), fitting in a compact `u8` with niche space and enabling single-register 24-bit level masks.
- Filter chains evaluate a tri-state outcome in registration order: `Reject` drops the record from the current `Dispatch`, `Accept` short-circuits remaining filters to support context-driven dynamic promotion ahead of downstream thresholds, and `Neutral` passes control onward (accepting the record if all filters finish `Neutral`).
- Each `Filter` reports its most permissive outcome per level (`Accept > Neutral > Reject`) at build time so `Dispatch` and `Logger` precompute static level bitmasks, pruning disabled callsites in a single bitwise test before `Record` construction while allowing runtime-mutable narrowing filters to return `Neutral` so dynamic updates take effect immediately.

## Structured Key-Values, Diagnostics, and Macros

- Synchronous paths borrow records, keys, and values on the caller stack while preserving `'static` string provenance to avoid copies; across asynchronous boundaries, records promote into `'static + Send + Sync` owned snapshots that preserve field insertion order and last-entry-wins duplicate-key lookup semantics.
- Native logging macros place bracketed `"key" = value` assignments ahead of `format_args!` as valid Rust expressions projected through `kv::LogValue`. Logger and level expressions evaluate once; enabled structured fields and format arguments evaluate at most once while borrowing non-`Copy` locals in place, and eager projection into `Send + Sync` borrowed values keeps enclosing `async` futures `Send` across `.await` points in macro arguments.
- Diagnostic traversal may run multiple times per record and must remain side-effect-free. Scoped thread-local and task-local diagnostics shadow outer entries for the same key and restore them automatically when exiting the scope.
- `Fatal` severities record event importance metadata only and neither flush buffers nor terminate the process.

## Error Handling and Operational Safety

- `Error` boxes its message and causal source chain so `Result<(), Error>` occupies a single pointer word on hot formatting, visiting, and appending paths.
- Logging pipelines must never panic on runtime operational failures such as out-of-range timestamps, broken pipes, closed descriptors, log rotation or retention errors, or fallible wire serialization. Leaf components return structured `Error`s (committing active record writes before surfacing post-write rollover or cleanup errors), execution roots and asynchronous workers isolate failures and panics via `ErrorSink`, and `ErrorSink` implementations silently absorb their own delivery failures without panicking or routing records back into their owning `Logger`.
- User `Display` and `Debug` formatting and value destruction execute outside appender I/O locks.
- Thread-local and task-local diagnostics access thread-local storage exclusively via fallible lookups (`LocalKey::try_with`) so logging during thread teardown degrades to an empty context instead of aborting.
- Because Rust does not run destructors on `static` items at process exit, buffered and asynchronous pipelines require an explicit `flush()` before shutdown.

## Ecosystem Interop

- Ecosystem bridges (such as `logforth-bridge-log` for the `log` facade) adapt external logging facades onto an explicit `Logger`, using `Logger::enabled` as a conservative, false-negative-free pre-check and deferring prefix or context filtering to each `Dispatch`.
- Libraries seeking ambient global logging emit via ecosystem facades directly, whereas libraries adopting Logforth natively accept an injected `Logger` defaulting to `Logger::noop()`.

## Appendix: Key-Value Naming Conventions

### Reserved Prefix (`logforth.*`)

- Keys prefixed with `logforth.` are reserved by Logforth. Adding new `logforth.*` keys in minor releases is SemVer-compatible; application code must not emit `logforth.*` keys for domain fields unless intentionally invoking a documented behavior.

| Key                 | Value Type | Producers                                  | Consumers and Semantics                                                 |
| :------------------ | :--------- | :----------------------------------------- | :---------------------------------------------------------------------- |
| `"logforth.target"` | `str`      | `LogBridge` (when `target != module_path`) | Matched by `RustLogFilter` before falling back to `Record::module_path` |

### Well-Known Shared Keys

- Unprefixed well-known keys preserve standard field names in external log outputs while coordinating typed context between diagnostics, layouts, and appenders:

| Key               | Value Type  | Producers            | Consumers and Semantics                                                             |
| :---------------- | :---------- | :------------------- | :---------------------------------------------------------------------------------- |
| `"trace_id"`      | `str` (hex) | `FastraceDiagnostic` | Extracted by `GoogleCloudLoggingLayout` into `logging.googleapis.com/trace`         |
| `"span_id"`       | `str` (hex) | `FastraceDiagnostic` | Extracted by `GoogleCloudLoggingLayout` into `logging.googleapis.com/spanId`        |
| `"sampled"`       | `bool`      | `FastraceDiagnostic` | Extracted by `GoogleCloudLoggingLayout` into `logging.googleapis.com/trace_sampled` |
| `"trace_sampled"` | `bool`      | User diagnostics     | Accepted by `GoogleCloudLoggingLayout` alongside `"sampled"`                        |

### Precedence and Flat-Format Envelope Fields

- When `Record::key_values()` and `Diagnostic` emit the same key, map-based outputs give precedence to explicit `Record` key-values over `Diagnostic` entries.
- Nested layouts isolate structured entries under dedicated sub-objects (`kvs` and `diags`), whereas flat layouts and protocol sinks emit record metadata and key-values in a single flat namespace; avoid reusing envelope field names such as `timestamp`, `level`, `severity`, `message`, `module`, or `module_path` when targeting flat sinks.
