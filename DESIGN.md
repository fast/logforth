# Architecture and 1.0 Design

This document defines the architectural model, public contracts, and target `1.0` design for `logforth-core` and its surrounding ecosystem crates. Runnable examples belong in [README.md](README.md) and crate-level API documentation.

## Scope and Crate Versioning

- Keep `logforth-core` at zero mandatory third-party dependencies so stabilizing `logforth-core 1.0` provides a permanent contract for `Logger`, `Record`, `kv`, and the five core extension traits without coupling SemVer stability to external SDKs.
- Evolve the `logforth` umbrella facade alongside peripheral appenders, layouts, filters, diagnostics, and ecosystem bridges on independent `0.x` tracks where dependencies such as `opentelemetry`, `jiff`, `fastrace`, `log`, and `tracing` change at their own pace.

## Pipeline Topology and Assembly

Logforth separates four concerns that legacy single-logger designs conflate into a single `target` string:

| Dimension            | Question Answered                                         | Representation in Logforth                                                                            |
| :------------------- | :-------------------------------------------------------- | :---------------------------------------------------------------------------------------------------- |
| Source location      | Where in the source tree was the event emitted?           | Compile-time `module_path`, `file`, `line`, and `column` on `Record`                                  |
| Category (`target`)  | Which logical module or subsystem emitted the event?      | Hierarchical string on `Record` (defaulting to `module_path!()`) used for verbosity filtering         |
| Channel (`Logger`)   | Which reliability, schema, and lifecycle stream owns it?  | Explicit `Logger` instances (for example, separating application diagnostics from `audit`/`metering`) |
| Fan-out (`Dispatch`) | Which output slices and destinations receive this stream? | Ordered `Dispatch` branches (`filters -> diagnostics -> appends`) inside a `Logger`                   |

- `Logger` owns a `Vec<Dispatch>` and a fallback `ErrorSink` without a global singleton in `logforth-core`. Callers share a `Logger` explicitly via `&Logger`, `Arc<Logger>`, or `&'static Logger`, and libraries accepting an optional logger can default to a zero-allocation `const fn Logger::noop()`.
- All five component traits (`Append`, `Layout`, `Filter`, `Diagnostic`, and `ErrorSink`) implement forwarding for `Arc<T>` and `&'static T`. Multiple dispatches or independent `Logger` instances can share stateful components such as `Arc<File>` or `Arc<Async>` as a directed acyclic graph without introducing a mandatory `LoggerProvider` ceremony.
- Configuration follows component semantics rather than uniform inheritance: `DispatchBuilder::error_sink` optionally overrides `LoggerBuilder::error_sink` (falling back when unset), while `Filter` and `Diagnostic` stay at the `Dispatch` level in `1.0` (shared via `Arc` when needed) so a `RUST_LOG` filter on console output cannot accidentally silence a sibling audit or OTLP dispatch. Out-of-band executors such as `Async` spawn their worker before joining a `Logger` and therefore configure their own `ErrorSink` (sharable via `Arc`).

## Core Extension Traits

Every long-lived pipeline extension point is an object-safe trait (`fmt::Debug + Send + Sync + 'static`) paired with concrete struct implementations rather than closed enums or ad-hoc function pointers:

```rust
pub trait Filter: fmt::Debug + Send + Sync + 'static {
    fn level_bounds(&self) -> FilterLevelBounds { FilterLevelBounds::ALL }
    fn enabled(&self, criteria: &FilterCriteria, diags: Diagnostics) -> FilterResult;
    fn matches(&self, record: &Record, diags: Diagnostics) -> FilterResult { ... }
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

- `Diagnostics<'a>` is a `Copy` opaque view abstracting over `&'a [Box<dyn Diagnostic>]`, `&'a [&'a dyn Diagnostic]`, a single stack `&'a dyn Diagnostic`, or chained scopes. Replacing `&[Box<dyn Diagnostic>]` decouples public traits from `Dispatch`'s storage and eliminates per-record heap boxing in `Async` workers.
- `Layout::format` appends formatted bytes into a caller-supplied `&mut Vec<u8>` rather than returning a fresh `Vec<u8>` or writing piecemeal into `&mut dyn io::Write`:
  1. Implementations must only append to `buf` (never clearing or reading prior bytes), and callers must truncate `buf` back to its starting length on `Err`.
  2. Record framing (such as trailing `\n`) belongs to the owning `Append`, not `Layout`.
  3. Synchronous appenders may pool thread-local `Vec<u8>` buffers outside I/O locks using `try_borrow_mut` (falling back to a fresh `Vec<u8>` on reentrant logging inside user `Display::fmt`).

## Filtering and Fast-Path Pruning

- `Level` defines 24 exhaustive severity variants (`#[repr(u8)]` `1..=24`, `Trace` through `Fatal4`) mapped 1-to-1 to OpenTelemetry `SeverityNumber` while leaving 232 niche values for compact enum layout.
- `FilterResult` retains Logback and `iptables` ternary semantics:
  - `Reject` drops the record immediately from the current `Dispatch`.
  - `Accept` short-circuits remaining filters in the current `Dispatch` and accepts the record immediately, enabling diagnostic-driven dynamic overrides (such as promoting a specific request or trace in `TaskLocalDiagnostic` to `Debug` ahead of a baseline `RustLogFilter`).
  - `Neutral` expresses no decision and passes control to the next filter; a `Dispatch` accepts the record if all filters finish `Neutral`.
- Two-phase invariant: `Filter::enabled` is a conservative pre-check before `Record` construction. If `enabled` returns `Reject`, `matches` must also reject; if `matches` can short-circuit with `Accept` based on payload or key-values, `enabled` must conservatively return `Accept` when followed by a stricter filter.
- `LevelMask(u32)` represents any subset of the 24 levels (including non-monotonic `LevelFilter` predicates such as `Equal` or `MoreVerbose`) in a single register. Each `Filter` reports static `(may_accept, may_neutral)` masks (`LevelFilter` and `RustLogFilter` report `may_accept = NONE`), and `DispatchBuilder::build` computes the dispatch's `LevelMask` via backwards induction `mask = may_accept | (may_neutral & next)`, folding pure `LevelFilter` items so common level-only dispatches allocate zero `Box<dyn Filter>` objects.
- `Logger::enabled` tests each dispatch's `LevelMask` with a single bitwise AND before invoking virtual filters. Attaching a leading diagnostic `Accept` override intentionally widens that dispatch's `LevelMask` to `ALL`, trading static level pruning for per-event context inspection on that branch; runtime-mutable filters likewise report `ALL` in `1.0`.

## Structured Key-Values and Diagnostics

- Borrowed-to-owned duality: synchronous paths carry stack-borrowed `Record<'a>`, `Key<'a>`, and `Value<'a>` (backed by `RefStr<'a>` to distinguish ephemeral `&'a str` borrows from zero-copy `&'static str` literals), whereas `Record::to_owned()` promotes records into `'static + Send` `RecordOwned`, `KeyOwned`, and `ValueOwned` snapshots that async workers re-project via `RecordOwned::with`.
- `ValueView<'a>` (`#[non_exhaustive]`) refines three contracts for `1.0`:
  1. Unify `BorrowedStr` and `StaticStr` into a single `ValueView::Str(StrValue<'a>)` variant exposing `as_str(&self) -> &'a str` and `as_static_str(&self) -> Option<&'static str>`, preventing `#[non_exhaustive]` wildcard arms in third-party layouts from silently mishandling static strings.
  2. Add first-class `Value::error(&'a (dyn StdError + 'static))` and `ValueView::Error(ErrorValue<'a>)` (exposing `as_std()` when borrowed and iterating/formatting the full `.source()` chain, promoted to `Box<[Cow<'static, str>]>` in `ValueOwned`) so structured JSON, GCP, OpenTelemetry, and bridge outputs preserve causal error chains.
  3. Expose `KeyView::as_static_str`, `ValueView::to_bytes`, and `ValueView::is_none`, plus `From<Option<T>>` and `From<&'a Cow<'_, str>>` conversions.
- Lossless owned storage: `ValueOwned` uses `Box<[u8]>`, `Box<[ValueOwned]>`, and insertion-ordered `Box<[(KeyOwned, ValueOwned)]>` (removing `HashMap` constructors) so synchronous and asynchronous paths preserve identical key ordering and duplicate-key semantics.
- Built-in diagnostics (`StaticDiagnostic`, `ThreadLocalDiagnostic` with RAII scope guards, and `TaskLocalDiagnostic`) uniformly store `(KeyOwned, ValueOwned)`. Diagnostic visitors may run multiple times per record and must remain side-effect-free; when keys collide in map-based outputs, explicit `Record` key-values take precedence over `Diagnostic` entries.

## Error Handling and Operational Safety

- `Error` boxes its payload (`Cow<'static, str>` message plus optional `Box<dyn StdError + Send + Sync + 'static>` source) so `Result<(), Error>` occupies a single machine word (`8` bytes on 64-bit targets) on hot `Visitor`, `Layout`, and `Append` return paths.
- Non-crashing runtime rule: logging pipelines must never `panic!` or `.unwrap()` on fallible user `Display`/`Debug` implementations, out-of-range timestamps, broken pipes (`EPIPE`), closed descriptors, or log rotation errors. Leaf components return `Err(Error)` with operation-specific messages; `RollingFileWriter` writes the current record to the active file before returning any rollover or cleanup error.
- Threading, TLS, and unwind safety:
  - User `Display`/`Debug` formatting always executes on the caller thread outside appender I/O mutexes (`Record::to_owned` snapshots payloads on the caller thread before enqueuing to `Async`).
  - `ThreadLocalDiagnostic` and `TaskLocalDiagnostic` access thread-local storage exclusively via `LocalKey::try_with` so logging from TLS destructors during thread teardown degrades gracefully to empty context instead of aborting.
  - `Async` workers guard per-record dispatch with `catch_unwind` (reporting panics via `ErrorSink`) and never `.expect()` when joining worker threads in `Drop`. Because Rust does not drop `static` items at process exit, global bridges require explicit `flush()` before shutdown.

## Native Logging Macros

```rust
logforth::info!(logger, "listening on {}", addr);
logforth::info!(logger, ["port" = port, tls = true], "server started");
logforth::info!(logger, ["jobs.completed" = count]);
logforth::info!(logger, target = "statement", ["rows" = n], "query finished");
```

- Tooling compatibility: structured fields use bracketed assignment expressions (`["k" = v, flag = v2]`) before the format string, and optional category overrides use `target = "..."` rather than `target: "..."`. Every argument is a valid Rust `ast::Expr`, allowing `rustfmt` to format the entire macro invocation cleanly without colliding with `format_args!` named parameters.
- Evaluation and `async` `Send` safety: `$logger` and `$level` evaluate exactly once. When `logger.enabled(&criteria)` succeeds, value expressions evaluate at most once and bind shared references (`let (v0, ...) = (&$v0, ...);`) before constructing `!Send` `Value<'_>` or `fmt::Arguments<'_>` temporaries, ensuring `Future`s that `.await` inside log arguments remain `Send`.
- Zero-copy keys and compact codegen: string literal keys (`"jobs.completed"`) and identifier shorthands (`tls`) expand to `Key::new` (`'static` provenance), and record construction delegates to a non-generic `__private_log` function backed by a `static` callsite location descriptor. `Level::Fatal` records severity metadata only and neither flushes nor terminates the process.

## Ecosystem Interop and Bridges

Logforth provides bidirectional adapters modeled after SLF4J so applications and libraries can adopt native `Logger` pipelines incrementally alongside `log` and `tracing`:

| Direction                                | Crate                     | Role                                                                                                                                             |
| :--------------------------------------- | :------------------------ | :----------------------------------------------------------------------------------------------------------------------------------------------- |
| Inbound (`log` -> `logforth`)            | `logforth-bridge-log`     | Implements `log::Log` (`LogBridge`), preserving `log::kv` types and setting `log::set_max_level` from `Logger`'s `LevelMask` across the 5 levels |
| Inbound (`tracing` -> `logforth`)        | `logforth-bridge-tracing` | Implements `tracing_subscriber::Layer`, translating `tracing::Event` fields (including `record_error`) and flattening `event_scope()` span kvs   |
| Outbound (`logforth` -> `log`/`tracing`) | `logforth-append-log`     | Implements `Append` by forwarding `Record` and `Diagnostics` to `log::logger()`, supporting both `log` hosts and `tracing-log` hosts             |

- Libraries adopting `logforth` natively can accept an injected `Logger` whose default routes to `logforth-append-log` (or `Logger::noop()`), remaining observable inside host applications that have not yet migrated to `logforth`.
- Loop prevention: if `LogBridge` is installed as the global `log` logger while a dispatch also attaches `logforth-append-log`, a panic-safe thread-local guard set by `logforth-append-log` and checked at `LogBridge` entry breaks the cycle, drops the reentrant record, and reports the misconfiguration once via `ErrorSink`.

## Registry Boundary

- `logforth-core` deliberately omits a global named logger registry. Global registries couple core APIs to initialization-order races, ambiguous shutdown/flush ownership, and configuration-driven type registration (`log4rs`-style dependency fan-in), while general-purpose crates on crates.io cannot assume a `logforth` registry exists.
- Applications requiring configuration-driven named routing, dynamic per-component reconfiguration, or administrative endpoints should layer an optional external `logforth-registry` crate purely atop `logforth-core`'s public primitives (`Arc<Logger>`, `Arc<T>` component sharing, and external atomic pointer swaps such as `ArcSwap`). Any registry requirement that cannot be expressed through public `logforth-core` traits indicates a missing composable primitive rather than a reason to embed a registry in `core`.
