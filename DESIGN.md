# Architecture and 1.0 Design

This document defines the architectural model, public contracts, and target `1.0` design for `logforth-core` and its surrounding ecosystem crates. Runnable examples belong in [README.md](README.md) and crate-level API documentation.

## Scope and Crate Versioning

- `logforth-core` carries zero mandatory third-party dependencies. Stabilizing `logforth-core 1.0` establishes a stable contract for `Logger`, `Record`, `kv`, and the five core extension traits without coupling SemVer stability to external SDKs.
- The `logforth` umbrella facade and peripheral appenders, layouts, filters, diagnostics, and ecosystem bridges evolve on independent `0.x` tracks so external dependencies such as `opentelemetry`, `jiff`, `fastrace`, `log`, and `tracing` can upgrade at their own pace.

## Pipeline Topology and Assembly

Logforth separates three concerns into distinct primitives:

| Dimension            | Question Answered                                         | Representation in Logforth                                    |
| :------------------- | :-------------------------------------------------------- | :------------------------------------------------------------ |
| Source location      | Where in the source tree was the event emitted?           | Compile-time `module_path`, `file`, `line`, and `column`      |
| Channel (`Logger`)   | Which reliability, schema, and lifecycle stream owns it?  | Explicit `Logger` instances                                   |
| Fan-out (`Dispatch`) | Which output slices and destinations receive this stream? | Ordered `Dispatch` branches inside a `Logger`                 |

- `Logger` is a lightweight `Clone` handle over a shared immutable pipeline, with no global singleton in `logforth-core`. Applications expose process-wide channels through their own initialized statics or dependency injection, while libraries accepting an optional logger can default to a zero-allocation `const fn Logger::noop()`.
- All five component traits (`Append`, `Layout`, `Filter`, `Diagnostic`, and `ErrorSink`) implement forwarding for `Arc<T>` and `&'static T` across all trait methods (with `Diagnostic` forwarding `&T` across any lifetime), allowing multiple dispatches or independent `Logger` instances to share stateful components directly as a directed acyclic graph; an `Append` or `ErrorSink` must never route back into its owning `Logger`.
- Configuration placement follows each component's role in the pipeline. `LoggerBuilder::error_sink` sets the logger-wide failure fallback, and `DispatchBuilder::error_sink` optionally overrides it for a specific branch. Out-of-band executors such as `Async` manage an independent worker lifecycle and configure their own `ErrorSink`. `Filter` and `Diagnostic` are configured on `DispatchBuilder` because branches within a `Logger` exist to select distinct event subsets and context; branches that share a filter or diagnostic share it explicitly via `Arc`.

For example, an application can isolate `audit` events on a dedicated `Logger` while its main diagnostic `Logger` fans out `Warn` records to standard error and `Debug` records to a shared `Arc<File>`.

## Core Extension Traits

Every long-lived pipeline extension point (`Filter`, `Layout`, `Append`, and `ErrorSink`) is an object-safe trait (`fmt::Debug + Send + Sync + 'static`) paired with concrete struct implementations rather than closed enums or ad-hoc function pointers. `Diagnostic` omits the `'static` supertrait bound so per-call and decorator contexts can borrow stack-resident data, while `Box<dyn Diagnostic>` enforces `'static` when registering diagnostics on a `Dispatch`:

```rust
pub trait Filter: fmt::Debug + Send + Sync + 'static {
    fn bounds(&self) -> FilterBounds { FilterBounds::ALL }
    fn filter(&self, record: &Record, diags: &dyn Diagnostic) -> FilterResult;
}

pub trait Diagnostic: fmt::Debug + Send + Sync {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error>;
}

pub trait Layout: fmt::Debug + Send + Sync + 'static {
    fn format(&self, record: &Record, diags: &dyn Diagnostic, buf: &mut Vec<u8>) -> Result<(), Error>;
}

pub trait Append: fmt::Debug + Send + Sync + 'static {
    fn append(&self, record: &Record, diags: &dyn Diagnostic) -> Result<(), Error>;
    fn flush(&self) -> Result<(), Error>;
}

pub trait ErrorSink: fmt::Debug + Send + Sync + 'static {
    fn sink(&self, err: &Error);
}
```

`Diagnostic` is composable: `&T` and `Arc<T>` forward references and shared ownership, `[T]`, `[T; N]`, `Vec<T>`, `[Box<dyn Diagnostic>]`, `[Box<dyn Diagnostic>; N]`, and `Vec<Box<dyn Diagnostic>>` compose homogeneous sequences, and `Option<T>`, `()` (empty context), and tuples up to 12 elements compose conditional and heterogeneous sources. Callers pass `&dyn Diagnostic` directly without wrapper enums or intermediate allocations, and decorating appenders or layouts can chain borrowed per-call context onto incoming `diags` as `&(diags, extra)`.

`Append` consumes structured `&Record` and `&dyn Diagnostic` directly so protocol-native sinks can encode typed fields without intermediate text serialization, whereas byte-stream sinks compose a `Layout` internally.

`Layout::format` appends formatted bytes into a caller-supplied `&mut Vec<u8>` rather than allocating a new buffer per record. Implementations must only append to `buf` without clearing or reading prior bytes, callers must truncate `buf` back to its starting length on `Err`, and record framing such as a trailing newline belongs to the owning `Append` rather than `Layout`. Synchronous appenders may pool thread-local `Vec<u8>` buffers outside I/O locks via `try_borrow_mut`, falling back to a fresh `Vec<u8>` if user `Display::fmt` logs reentrantly.

## Filtering and Fast-Path Pruning

- `Level` defines 24 exhaustive severity variants with `#[repr(u8)]` discriminants `1..=24` from `Trace` through `Fatal4`, aligned 1-to-1 with OpenTelemetry `SeverityNumber` while leaving 232 niche values for compact enum layout.
- `FilterResult` models an ordered decision chain with three outcomes:
  - `Reject` drops the record immediately from the current `Dispatch`.
  - `Accept` short-circuits remaining filters in the current `Dispatch` and accepts the record immediately, supporting context-driven dynamic promotion ahead of baseline level thresholds.
  - `Neutral` expresses no decision and passes control to the next filter; a `Dispatch` accepts the record if all filters finish `Neutral`.
- `LevelMask(u32)` represents any subset of the 24 levels, including non-monotonic `LevelFilter` predicates, in a single machine register. Each `Filter` reports static `may_accept` and `may_neutral` masks through `Filter::bounds`, and `DispatchBuilder::build` computes the branch `LevelMask` by backwards induction `mask = may_accept | (may_neutral & next)`.
- `Logger::enabled(level)` tests the union of its dispatches' `LevelMask` values in a single bitwise operation before a callsite constructs a `Record`, and `Logger::log` tests each branch's `LevelMask` before evaluating its `Filter` chain. Attaching a leading context-driven `Accept` filter widens that branch's `LevelMask` to `ALL`, trading static level pruning for per-event context inspection on that branch; runtime-mutable filters likewise report `ALL` so dynamic updates take effect immediately.
- `RustLogFilter` filters on `Record::module_path()` after passing the level mask, and exposes named-directive filtering so custom filters can inspect explicit key-value fields before falling back to module paths.

## Structured Key-Values and Diagnostics

- `Record<'a>` carries `time`, `level`, `module_path`, `file`, `line`, `column`, `payload`, and a zero-allocation `KeyValues<'a>` slice.
- Synchronous paths borrow `Record<'a>`, `Key<'a>`, and `Value<'a>` on the caller stack, using `RefStr<'a>` to distinguish ephemeral `&'a str` borrows from zero-copy `&'static str` literals. Across asynchronous boundaries, `Record::to_owned()` promotes records into `'static + Send` `RecordOwned`, `KeyOwned`, and `ValueOwned` snapshots that workers re-project via `RecordOwned::with`.
- `Value<'a>` is a `Copy + Send + Sync` borrowed carrier providing `From` conversions for scalars, strings including `Cow<str>`, byte slices, `Option<T>`, and `&'a ValueOwned`, alongside `Value::debug` and `Value::display` constructors over `Sync` references; workspace types borrowing internal state can implement `From<&'a T> for Value<'a>` directly. `ValueView<'a>` is a `Copy`, `#[non_exhaustive]` enum that unifies read-only projection across `Value<'a>` and `ValueOwned`, implementing `fmt::Display` so unhandled variants degrade cleanly to strings while `KeyView` and `ValueView` expose typed scalar, byte, container, and `'static` string accessors.
- `ValueOwned` is `'static + Send + Sync` and stores `Cow<'static, str>`, `Box<[u8]>`, `Box<[ValueOwned]>`, and insertion-ordered `Box<[(KeyOwned, ValueOwned)]>` slices so synchronous and asynchronous paths preserve identical field ordering and duplicate-key semantics. On enabled callsites, nested structures that cannot be borrowed directly construct a temporary `ValueOwned` via custom constructors or the optional `serde` feature's `ValueOwned::from_serde` and `ValueOwned::try_from_serde`, then borrow into `KeyValues<'a>` through `From<&'a ValueOwned> for Value<'a>`.
- `StaticDiagnostic`, `ThreadLocalDiagnostic` with `!Send` RAII scope guards that restore shadowed values on drop, and `TaskLocalDiagnostic` store `(KeyOwned, ValueOwned)`. `Diagnostic::visit` may run multiple times per record and must remain side-effect-free; when keys collide in map-based outputs, explicit `Record` key-values take precedence over `Diagnostic` entries.

## Error Handling and Operational Safety

- `Error` boxes its `Cow<'static, str>` message and optional `Box<dyn StdError + Send + Sync + 'static>` cause so `Result<(), Error>` occupies a single word on hot `Visitor`, `Layout`, and `Append` return paths.
- Logging pipelines must never `panic!` or `.unwrap()` on fallible user `Display` or `Debug` implementations, out-of-range timestamps, broken pipes, closed descriptors, or log rotation failures. Infallible snapshot conversions such as `Record::to_owned` and `ValueView::to_owned` format via `fmt::write` and substitute a placeholder string on `fmt::Error` instead of calling `to_string()` or `format!`. Leaf components return `Err(Error)` with operation-specific messages, `RollingFileWriter` writes the current record to the active file before surfacing any rollover or cleanup error, and `ErrorSink` implementations operate on a best-effort basis that silently absorbs internal delivery failures without panicking.
- User `Display` and `Debug` formatting always runs on the caller thread outside appender I/O mutexes.
- `ThreadLocalDiagnostic` and `TaskLocalDiagnostic` access thread-local storage exclusively via `LocalKey::try_with` so logging from TLS destructors during thread teardown degrades to empty context instead of aborting.
- `Async` workers isolate per-record dispatch with `catch_unwind`, report panics via `ErrorSink`, and avoid `.expect()` when joining worker threads in `Drop`. Because Rust does not run destructors on `static` items at process exit, global bridges require an explicit `flush()` before shutdown.

## Native Logging Macros

```rust
logforth::info!(logger, "listening on {}", addr);
logforth::info!(logger, ["port" = port, "tls" = true], "server started");
logforth::info!(logger, ["jobs.completed" = count, "error" = Value::debug(&err)]);
```

- Structured fields precede the format string using bracketed `"key" = value` assignment expressions with string literal keys and plain Rust value expressions rather than sigils or implicit field shorthands. Every macro argument is a valid Rust `ast::Expr` so `rustfmt` formats the entire invocation without colliding with `format_args!` named parameters.
- `$logger` and `$level` evaluate once. When `logger.enabled(level)` succeeds, value and format-argument expressions evaluate at most once inside a `match` scrutinee that binds shared references before constructing `!Send` `fmt::Arguments<'_>` inside the synchronous `match` arm; because `Value<'_>` itself is `Send + Sync`, temporaries stay alive, non-`Copy` locals remain unmoved, and enclosing `async` futures stay `Send` across `.await` points inside log arguments.
- String literal keys expand to `Key::new` with `'static` provenance, and record construction delegates to a non-generic `__private_log` helper backed by a `static` callsite location descriptor. `Level::Fatal` records severity metadata only and neither flushes nor terminates the process.

## Ecosystem Interop and Bridges

Logforth provides bidirectional adapters for `log` and `tracing` so applications and libraries can adopt native `Logger` pipelines incrementally:

| Direction                                | Crate                     | Role                                                                                                                                             |
| :--------------------------------------- | :------------------------ | :----------------------------------------------------------------------------------------------------------------------------------------------- |
| Inbound (`log` -> `logforth`)            | `logforth-bridge-log`     | Implements `log::Log`, forwarding structured key-values and updating `log::set_max_level` from `Logger`'s `LevelMask` on global installation    |
| Inbound (`tracing` -> `logforth`)        | `logforth-bridge-tracing` | Implements `tracing_subscriber::Layer`, translating `tracing::Event` fields and flattening active `event_scope()` span attributes into `Record` |
| Outbound (`logforth` -> `log`/`tracing`) | `logforth-append-log`     | Implements `Append` by forwarding `Record` and `&dyn Diagnostic` to `log::logger()`, supporting both `log` hosts and `tracing-log` hosts        |

- Inbound `log` and `tracing` bridges populate `Record::module_path` from the event's module path and emit a `"target"` key-value entry only when the upstream caller explicitly overrode `target` to differ from `module_path`. `LogBridge::enabled` delegates directly to `Logger::enabled` on the mapped severity level.
- Libraries adopting `logforth` natively can accept an injected `Logger` whose default routes to `logforth-append-log` or `Logger::noop()`, remaining observable inside host applications that use another subscriber.
- If an inbound `log` or `tracing` bridge feeds a `Logger` whose dispatch also attaches `logforth-append-log`, a panic-safe thread-local guard in `logforth-core` set by `logforth-append-log` and checked at inbound bridge entry breaks the cycle, drops the reentrant record, and reports the misconfiguration once via `ErrorSink`.
