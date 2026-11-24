# Architecture and 1.0 Design

This document defines the architectural model, public contracts, and target `1.0` design for `logforth-core` and its surrounding ecosystem crates. Runnable examples belong in [README.md](README.md) and crate-level API documentation.

## Scope and Crate Versioning

- `logforth-core` carries zero mandatory third-party dependencies. Stabilizing `logforth-core 1.0` establishes a permanent contract for `Logger`, `Record`, `kv`, and the five core extension traits without coupling SemVer stability to external SDKs.
- The `logforth` umbrella facade and peripheral appenders, layouts, filters, diagnostics, and ecosystem bridges evolve on independent `0.x` tracks so external dependencies such as `opentelemetry`, `jiff`, `fastrace`, `log`, and `tracing` can upgrade at their own pace.

## Pipeline Topology and Assembly

Logforth separates three concerns into distinct primitives:

| Dimension            | Question Answered                                         | Representation in Logforth                                    |
| :------------------- | :-------------------------------------------------------- | :------------------------------------------------------------ |
| Source location      | Where in the source tree was the event emitted?           | Compile-time `module_path`, `file`, `line`, and `column`      |
| Channel (`Logger`)   | Which reliability, schema, and lifecycle stream owns it?  | Explicit `Logger` instances                                   |
| Fan-out (`Dispatch`) | Which output slices and destinations receive this stream? | Ordered `Dispatch` branches inside a `Logger`                 |

- `Logger` is a lightweight `Clone` handle backed by a shared pipeline and an inline per-handle level gate, without a global singleton in `logforth-core`. Libraries accepting an optional logger can default to a zero-allocation `const fn Logger::noop()`, and derived handles created with static or atomic `LevelMask` gates share the same underlying dispatches and `flush` lifecycle while enforcing independent `O(1)` callsite thresholds.
- All five component traits (`Append`, `Layout`, `Filter`, `Diagnostic`, and `ErrorSink`) implement forwarding for `Arc<T>` and `&'static T`. Multiple dispatches or independent `Logger` instances can share stateful components as a directed acyclic graph without a separate provider wrapper.
- Configuration placement follows each component's role in the pipeline. `LoggerBuilder::error_sink` sets the logger-wide failure fallback, and `DispatchBuilder::error_sink` optionally overrides it for a specific branch. Out-of-band executors such as `Async` manage an independent worker lifecycle and configure their own `ErrorSink`. `Filter` and `Diagnostic` are configured on `DispatchBuilder` because branches within a `Logger` exist to select distinct event subsets and context; branches that share a filter or diagnostic share it explicitly via `Arc`.

For example, an application can isolate `audit` events on a dedicated `Logger` while its main diagnostic `Logger` fans out `Warn` records to standard error and `Debug` records to a shared `Arc<File>`.

## Core Extension Traits

Every long-lived pipeline extension point is an object-safe trait (`fmt::Debug + Send + Sync + 'static`) paired with concrete struct implementations rather than closed enums or ad-hoc function pointers:

```rust
pub trait Filter: fmt::Debug + Send + Sync + 'static {
    fn bounds(&self) -> FilterBounds { FilterBounds::ALL }
    fn check(&self, record: &Record, diags: Diagnostics) -> FilterResult;
}

pub trait Diagnostic: fmt::Debug + Send + Sync + 'static {
    fn visit(&self, visitor: &mut dyn kv::Visitor) -> Result<(), Error>;
}

pub trait Layout: fmt::Debug + Send + Sync + 'static {
    fn format(&self, record: &Record, diags: Diagnostics, buf: &mut Vec<u8>) -> Result<(), Error>;
}

pub trait Append: fmt::Debug + Send + Sync + 'static {
    fn append(&self, record: &Record, diags: Diagnostics) -> Result<(), Error>;
    fn flush(&self) -> Result<(), Error>;
}

pub trait ErrorSink: fmt::Debug + Send + Sync + 'static {
    fn sink(&self, err: &Error);
}
```

`Diagnostics<'a>` is a `Copy` view passed to filters, layouts, and appenders without exposing internal container types:

```rust
#[derive(Clone, Copy)]
pub struct Diagnostics<'a>(DiagnosticsInner<'a>);

#[derive(Clone, Copy)]
enum DiagnosticsInner<'a> {
    Empty,
    // Borrows a Dispatch's owned diagnostic list without copying or re-boxing.
    Boxed(&'a [Box<dyn Diagnostic>]),
    // Borrows a stack-resident diagnostic, such as materialized context in an async worker.
    Single(&'a dyn Diagnostic),
    // Borrows a slice of trait-object references for ad-hoc composition and testing.
    Slice(&'a [&'a dyn Diagnostic]),
}
```

- `Append` consumes structured `&Record` and `Diagnostics` directly so protocol-native sinks can encode typed fields without intermediate text serialization, whereas byte-stream sinks compose a `Layout` internally.
- `Layout::format` appends formatted bytes into a caller-supplied `&mut Vec<u8>`:
  1. Implementations must only append to `buf` without clearing or reading prior bytes, and callers must truncate `buf` back to its starting length on `Err`.
  2. Record framing such as a trailing newline belongs to the owning `Append`, not `Layout`.
  3. Synchronous appenders may pool thread-local `Vec<u8>` buffers outside I/O locks via `try_borrow_mut`, falling back to a fresh `Vec<u8>` if user `Display::fmt` logs reentrantly.

## Filtering and Fast-Path Pruning

- `Level` defines 24 exhaustive severity variants with `#[repr(u8)]` discriminants `1..=24` from `Trace` through `Fatal4`, aligned 1-to-1 with OpenTelemetry `SeverityNumber` while leaving 232 niche values for compact enum layout.
- `FilterResult` models an ordered decision chain with three outcomes:
  - `Reject` drops the record immediately from the current `Dispatch`.
  - `Accept` short-circuits remaining filters in the current `Dispatch` and accepts the record immediately, supporting context-driven dynamic promotion ahead of baseline level thresholds.
  - `Neutral` expresses no decision and passes control to the next filter; a `Dispatch` accepts the record if all filters finish `Neutral`.
- `LevelMask(u32)` represents any subset of the 24 levels, including non-monotonic `LevelFilter` predicates, in a single machine register. Each `Filter` reports static `may_accept` and `may_neutral` masks through `Filter::bounds`. `DispatchBuilder::build` computes the branch `LevelMask` by backwards induction `mask = may_accept | (may_neutral & next)` and folds pure `LevelFilter` items directly into the mask without retaining `Box<dyn Filter>` allocations.
- `Logger::enabled(level)` tests the handle's level gate and the underlying dispatches' `LevelMask` union with bitwise AND operations before a callsite constructs a `Record`. Attaching a leading context-driven `Accept` filter widens that branch's `LevelMask` to `ALL`, trading static level pruning for per-event context inspection on that branch; runtime-mutable filters likewise report `ALL` so dynamic updates take effect immediately.
- Module-level switching operates at two granularities without scanning `Record::key_values()`:
  1. Per-subsystem or per-module `Logger` handles, passed explicitly or cached in a module `LazyLock<Logger>` resolved once at initialization, achieve `O(1)` callsite pruning with zero string matching; dynamic reconfiguration updates each handle's `Arc<AtomicLevelMask>` on the cold control path.
  2. A single shared `Logger` paired with `RustLogFilter` reads `Record::module_path()` in `O(1)` after passing the level mask, and `RustLogFilter` exposes a module-matching method so custom filters can compose directive matching on arbitrary fields without adding KV-scanning options to `RustLogFilter`.

## Structured Key-Values and Diagnostics

- `Record<'a>` carries `time`, `level`, `module_path`, `file`, `line`, `column`, `payload`, and a zero-allocation `KeyValues<'a>` slice.
- Synchronous paths borrow `Record<'a>`, `Key<'a>`, and `Value<'a>` on the caller stack, using `RefStr<'a>` to distinguish ephemeral `&'a str` borrows from zero-copy `&'static str` literals. Across asynchronous boundaries, `Record::to_owned()` promotes records into `'static + Send` `RecordOwned`, `KeyOwned`, and `ValueOwned` snapshots that workers re-project via `RecordOwned::with`.
- `Value<'a>` provides `From` conversions for scalars, strings including `Cow<str>`, byte slices, and `Option<T>`, alongside explicit `Value::debug` and `Value::display` constructors. `ValueView<'a>` is marked `#[non_exhaustive]` and implements `fmt::Display` so unhandled variants degrade cleanly to strings, while `KeyView` and `ValueView` expose typed scalar, byte, container, and `'static` string accessors.
- `ValueOwned` stores `Box<[u8]>`, `Box<[ValueOwned]>`, and insertion-ordered `Box<[(KeyOwned, ValueOwned)]>` slices so synchronous and asynchronous paths preserve identical field ordering and duplicate-key semantics.
- `StaticDiagnostic`, `ThreadLocalDiagnostic` with RAII scope guards, and `TaskLocalDiagnostic` store `(KeyOwned, ValueOwned)`. `Diagnostic::visit` may run multiple times per record and must remain side-effect-free; when keys collide in map-based outputs, explicit `Record` key-values take precedence over `Diagnostic` entries.

## Error Handling and Operational Safety

- `Error` boxes its `Cow<'static, str>` message and optional `Box<dyn StdError + Send + Sync + 'static>` cause so `Result<(), Error>` occupies a single word on hot `Visitor`, `Layout`, and `Append` return paths.
- Logging pipelines must never `panic!` or `.unwrap()` on fallible user `Display` or `Debug` implementations, out-of-range timestamps, broken pipes, closed descriptors, or log rotation failures. Leaf components return `Err(Error)` with operation-specific messages, and `RollingFileWriter` writes the current record to the active file before surfacing any rollover or cleanup error.
- User `Display` and `Debug` formatting always runs on the caller thread outside appender I/O mutexes.
- `ThreadLocalDiagnostic` and `TaskLocalDiagnostic` access thread-local storage exclusively via `LocalKey::try_with` so logging from TLS destructors during thread teardown degrades to empty context instead of aborting.
- `Async` workers isolate per-record dispatch with `catch_unwind`, report panics via `ErrorSink`, and avoid `.expect()` when joining worker threads in `Drop`. Because Rust does not run destructors on `static` items at process exit, global bridges require an explicit `flush()` before shutdown.

## Native Logging Macros

```rust
logforth::info!(logger, "listening on {}", addr);
logforth::info!(logger, ["port" = port, tls = true], "server started");
logforth::info!(logger, ["jobs.completed" = count]);
```

- Structured fields precede the format string using bracketed assignment expressions. Every macro argument is a valid Rust `ast::Expr` so `rustfmt` formats the entire invocation without colliding with `format_args!` named parameters.
- `$logger` and `$level` evaluate once. When `logger.enabled(level)` succeeds, value expressions evaluate at most once and bind shared references before constructing `!Send` `Value<'_>` or `fmt::Arguments<'_>` temporaries, keeping enclosing `async` futures `Send` across `.await` points inside log arguments.
- String literal keys and identifier shorthands expand to `Key::new` with `'static` provenance, and record construction delegates to a non-generic `__private_log` helper backed by a `static` callsite location descriptor. `Level::Fatal` records severity metadata only and neither flushes nor terminates the process.

## Ecosystem Interop and Bridges

Logforth provides bidirectional adapters for `log` and `tracing` so applications and libraries can adopt native `Logger` pipelines incrementally:

| Direction                                | Crate                     | Role                                                                                                                                             |
| :--------------------------------------- | :------------------------ | :----------------------------------------------------------------------------------------------------------------------------------------------- |
| Inbound (`log` -> `logforth`)            | `logforth-bridge-log`     | Implements `log::Log`, mapping `log::Record::target()` to `Record::module_path` and setting `log::set_max_level` from `Logger`'s `LevelMask`     |
| Inbound (`tracing` -> `logforth`)        | `logforth-bridge-tracing` | Implements `tracing_subscriber::Layer`, translating `tracing::Event` fields and flattening active `event_scope()` span attributes into `Record`  |
| Outbound (`logforth` -> `log`/`tracing`) | `logforth-append-log`     | Implements `Append` by forwarding `Record` and `Diagnostics` to `log::logger()`, supporting both `log` hosts and `tracing-log` hosts             |

- `LogBridge::enabled` delegates directly to `Logger::enabled` on the mapped severity level without constructing a synthetic `Record`.
- Libraries adopting `logforth` natively can accept an injected `Logger` whose default routes to `logforth-append-log` or `Logger::noop()`, remaining observable inside host applications that use another subscriber.
- If `LogBridge` is installed as the global `log` logger while a dispatch also attaches `logforth-append-log`, a panic-safe thread-local guard set by `logforth-append-log` and checked at `LogBridge` entry breaks the cycle, drops the reentrant record, and reports the misconfiguration once via `ErrorSink`.

## Registry Boundary

- `logforth-core` omits a global named logger registry to avoid initialization-order races, ambiguous shutdown and flush ownership, and configuration-driven dependency fan-in, as crates on crates.io cannot assume a `logforth` registry exists.
- Applications requiring configuration-driven named routing, acquisition-time `O(1)` module level resolution, or runtime reconfiguration layer an optional external `logforth-registry` crate atop `logforth-core`'s public primitives (`Logger` handle derivation, `Arc<AtomicLevelMask>`, `Arc<T>` component sharing, and atomic pointer swaps). Any registry requirement that cannot be expressed through public `logforth-core` traits indicates a missing composable primitive rather than a reason to embed a registry in `core`.
