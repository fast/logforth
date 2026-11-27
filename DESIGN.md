# Architecture and 1.0 Design

This document defines the architectural model, public contracts, and target `1.0` design for `logforth-core` and its surrounding ecosystem crates. Runnable examples belong in [README.md](README.md) and crate-level API documentation.

## Scope and Crate Versioning

- `logforth-core` carries zero mandatory third-party dependencies. Stabilizing `logforth-core 1.0` establishes a stable contract for `Logger`, `Record`, `kv`, and the five core extension traits without coupling SemVer stability to external SDKs.
- The `logforth` umbrella facade and peripheral appenders, layouts, filters, diagnostics, and ecosystem bridges evolve on independent `0.x` tracks so external dependencies such as `opentelemetry`, `jiff`, `fastrace`, and `log` can upgrade at their own pace.

## Pipeline Topology and Assembly

Logforth separates three concerns into distinct primitives:

| Dimension            | Question Answered                                         | Representation in Logforth                                    |
| :------------------- | :-------------------------------------------------------- | :------------------------------------------------------------ |
| Source location      | Where in the source tree was the event emitted?           | Compile-time `module_path`, `file`, `line`, and `column`      |
| Channel (`Logger`)   | Which reliability, schema, and lifecycle stream owns it?  | Explicit `Logger` instances                                   |
| Fan-out (`Dispatch`) | Which output slices and destinations receive this stream? | Ordered `Dispatch` branches inside a `Logger`                 |

- `Logger` is a lightweight `Clone` handle over a shared immutable pipeline, with no global singleton in `logforth-core`. Applications expose process-wide channels through their own initialized statics or dependency injection, while libraries accepting an optional logger can default to a zero-allocation `const fn Logger::noop()`.
- All five component traits (`Append`, `Layout`, `Filter`, `Diagnostic`, and `ErrorSink`) implement forwarding for `Arc<T>` and `&'static T` across all trait methods (with `Diagnostic` forwarding `&T` across any lifetime), allowing multiple dispatches or independent `Logger` instances to share stateful components directly as a directed acyclic graph; an `Append` or `ErrorSink` must never route back into its owning `Logger`.
- Configuration placement follows each component's role in the pipeline. `LoggerBuilder::error_sink` sets the logger-wide failure fallback, and `DispatchBuilder::error_sink` optionally overrides it for a specific branch. Out-of-band executors such as `Async` manage an independent worker lifecycle and configure their own `ErrorSink`. `Filter` and `Diagnostic` are configured on `DispatchBuilder` because branches within a `Logger` exist to select distinct event subsets and context; branches that share a filter or diagnostic share it explicitly via `Arc`. A `DispatchBuilder` configures a single branch as a unit regardless of method call order, and branches configured with no appenders are skipped during `Logger` construction.

For example, an application can isolate `audit` events on a dedicated `Logger` while its main diagnostic `Logger` fans out `Warn` records to standard error and `Debug` records to a shared `Arc<File>`.

## Core Extension Traits

Every long-lived pipeline extension point (`Filter`, `Layout`, `Append`, and `ErrorSink`) is an object-safe trait (`fmt::Debug + Send + Sync + 'static`) paired with concrete struct implementations rather than closed enums or ad-hoc function pointers. `Diagnostic` omits the `'static` supertrait bound so per-call and decorator contexts can borrow stack-resident data, while `Box<dyn Diagnostic>` enforces `'static` when registering diagnostics on a `Dispatch`:

```rust
pub trait Filter: fmt::Debug + Send + Sync + 'static {
    fn filter_level(&self, level: Level) -> FilterResult;
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

`Diagnostic` is composable: `&T` and `Arc<T>` forward references and shared ownership, while `()` (empty context), `Option<T>`, `[T]`, `[T; N]`, and `Vec<T>` (along with the matching `Box<dyn Diagnostic>` containers) compose optional and sequential sources. Callers pass `&dyn Diagnostic` directly without wrapper enums or intermediate allocations, and decorating appenders or layouts can chain borrowed per-call context onto incoming `diags` as `&[diags, &extra]`.

`Append` consumes structured `&Record` and `&dyn Diagnostic` directly so protocol-native sinks can encode typed fields without intermediate text serialization, whereas byte-stream sinks compose a `Layout` internally.

`Layout::format` appends formatted bytes into a caller-supplied `&mut Vec<u8>` rather than allocating a new buffer per record. Implementations must only append to `buf` without modifying or removing prior bytes, callers reusing `buf` must discard partial output on `Err`, and record framing such as a trailing newline belongs to the owning `Append` rather than `Layout`.

## Filtering and Fast-Path Pruning

- `Level` defines 24 exhaustive severity variants with `#[repr(u8)]` discriminants `1..=24` from `Trace` through `Fatal4`, aligned 1-to-1 with OpenTelemetry `SeverityNumber` while leaving 232 niche values for compact enum layout.
- `FilterResult` models an ordered decision chain with three outcomes:
  - `Reject` drops the record immediately from the current `Dispatch`.
  - `Accept` short-circuits remaining filters in the current `Dispatch` and accepts the record immediately, supporting context-driven dynamic promotion ahead of baseline level thresholds.
  - `Neutral` expresses no decision and passes control to the next filter; a `Dispatch` accepts the record if all filters finish `Neutral`.
- Each `Filter` reports the most permissive outcome (`Accept > Neutral > Reject`) that `Filter::filter` may produce at each `Level` through `Filter::filter_level(level)`. When a `Logger` is built, each `Dispatch` evaluates its `Filter` chain across all 24 levels once and caches the accepted subset (including non-monotonic `LevelFilter` predicates) in a single-register internal bitmask.
- `Logger::enabled(level)` tests the union of its dispatches' level masks in a single bitwise operation before a callsite constructs a `Record`, and `Logger::log` tests each branch's level mask before evaluating its `Filter` chain. Attaching a leading context-driven `Accept` filter enables the promoted levels on that branch, trading static level pruning for per-event context inspection while retaining downstream `LevelFilter` items to reject non-promoted records; runtime-mutable narrowing filters report `Neutral` from `Filter::filter_level` so dynamic updates take effect immediately.
- `RustLogFilter` matches an explicit `"logforth.target"` key-value entry when present (preserving `RUST_LOG` target directives such as `my_app::db=warn` for bridged `log` records without colliding with user `"target"` fields) before falling back to `Record::module_path()`, and fails fast with `Err(Error)` on malformed directive specifications or non-UTF-8 environment variables.

## Structured Key-Values and Diagnostics

- `Record<'a>` carries `time`, `level`, `module_path`, `file`, `line`, `column`, `payload`, and a zero-allocation `KeyValues<'a>` slice.
- Synchronous paths borrow `Record<'a>`, `Key<'a>`, and `Value<'a>` on the caller stack, using `RefStr<'a>` to distinguish ephemeral `&'a str` borrows from zero-copy `&'static str` literals. Across asynchronous boundaries, `Record::to_owned()` promotes records into `'static + Send` `RecordOwned`, `KeyOwned`, and `ValueOwned` snapshots that workers re-project via `RecordOwned::with`.
- `Value<'a>` is a `Copy + Send + Sync` borrowed carrier providing `From` conversions for scalars, strings including `Cow<str>`, byte slices, `Option<T>`, and `&'a ValueOwned`, alongside `Value::debug` and `Value::display` constructors over `Sync` references; workspace types borrowing internal state can implement `From<&'a T> for Value<'a>` directly. `ValueView<'a>` is a `Copy`, `#[non_exhaustive]` enum that unifies read-only projection across `Value<'a>` and `ValueOwned`, implementing `fmt::Display` so unhandled variants degrade cleanly to strings while `KeyView` and `ValueView` expose typed scalar, byte, container, and `'static` string accessors.
- `ValueOwned` is `'static + Send + Sync` and stores `Cow<'static, str>`, `Box<[u8]>`, `Box<[ValueOwned]>`, and insertion-ordered `Box<[(KeyOwned, ValueOwned)]>` slices so synchronous and asynchronous paths preserve identical field ordering and duplicate-key semantics. On enabled callsites, nested structures that cannot be borrowed directly construct a temporary `ValueOwned` via custom constructors or the optional `serde` feature's `ValueOwned::from_serde` and `ValueOwned::try_from_serde`, then borrow into `KeyValues<'a>` through `From<&'a ValueOwned> for Value<'a>`.
- `StaticDiagnostic`, `ThreadLocalDiagnostic` with `!Send` RAII scope guards that restore shadowed values on drop, and `TaskLocalDiagnostic` store `(KeyOwned, ValueOwned)`. `Diagnostic::visit` may run multiple times per record and must remain side-effect-free; when keys collide in map-based outputs, explicit `Record` key-values take precedence over `Diagnostic` entries.

## Error Handling and Operational Safety

- `Error` boxes its `Cow<'static, str>` message and optional `Box<dyn StdError + Send + Sync + 'static>` cause so `Result<(), Error>` occupies a single word on hot `Visitor`, `Layout`, and `Append` return paths.
- Logging pipelines must never `panic!` or `.unwrap()` on runtime operational failures such as out-of-range timestamps, broken pipes, closed descriptors, log rotation errors, or fallible protocol serialization; in-memory `Display` and `Debug` formatting follows the standard `core::fmt` contract. Leaf components return `Err(Error)` with operation-specific messages, `RollingFileWriter` writes the current record to the active file before surfacing any rollover or cleanup error, and `ErrorSink` implementations operate on a best-effort basis that silently absorbs internal delivery failures without panicking.
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

`logforth-bridge-log` adapts the `log` crate into a `Logger` so applications can adopt native `Logger` pipelines while continuing to collect logs from ecosystem dependencies:

- `LogBridge` implements `log::Log`, forwarding structured key-values and updating `log::set_max_level` from `Logger::enabled` on global installation.
- `LogBridge` populates `Record::module_path` from the event's module path and emits a `"logforth.target"` key-value entry only when the upstream caller explicitly overrode `target` to differ from `module_path`.
- `LogBridge::enabled` delegates to `Logger::enabled` on the mapped severity level as a conservative, false-negative-free pre-check for `log::log_enabled!`; per-prefix filtering executes inside each `Dispatch` when `Log::log` forwards the record.
- Libraries seeking ambient global logging emit via `log` directly, whereas libraries adopting `logforth` natively accept an injected `Logger` defaulting to `Logger::noop()`.

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
- Nested layouts such as `JsonLayout` isolate structured entries under `kvs` and `diags`, whereas flat layouts and protocol sinks (`LogfmtLayout`, `GoogleCloudLoggingLayout`, `OpentelemetryLog`, `FastraceEvent`, and `Journald`) emit record metadata and key-values in a single flat namespace; avoid reusing envelope field names such as `timestamp`, `level`, `severity`, `message`, `module`, or `module_path` when targeting flat sinks.
