# Repository Guidelines

This document is the single source of truth for workspace structure, development commands, Rust style, documentation contracts, testing, changelog rules, and pull request conventions for both human contributors and coding agents. Keep documented workflows synchronized with structural changes, and run `cargo x build`, `cargo x test`, and `cargo x lint` before submitting or handing work back.

For architectural direction, pipeline semantics, and the target `1.0` design, see [DESIGN.md](DESIGN.md).

## Workspace Layout

| Crate           | Purpose                                                                                                                           |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `core`          | Zero-mandatory-dependency core library (`logforth-core`): `Logger`, `Record`, `kv`, native macros, and the five extension traits. |
| `logforth`      | Umbrella facade crate re-exporting `logforth-core`, optional peripheral crates behind feature flags, and `starter_log`.           |
| `appenders/*`   | Built-in appender crates (`async`, `fastrace`, `file`, `journald`, `opentelemetry`, `syslog`).                                    |
| `layouts/*`     | Built-in layout crates (`google-cloud-logging`, `json`, `logfmt`, `text`).                                                        |
| `filters/*`     | Built-in filter crates (`rustlog`).                                                                                               |
| `diagnostics/*` | Built-in diagnostic context crates (`fastrace`, `task-local`).                                                                    |
| `bridges/*`     | Ecosystem bridge crates (`log`).                                                                                                  |
| `examples`      | Standalone runnable end-to-end scenarios (`cargo run --example ...`).                                                             |
| `xtask` (`x`)   | Workspace automation entrypoint exposed via the `cargo x` alias defined in `.cargo/config.toml`.                                  |

## Workflows

Use `cargo x` as the single entrypoint for repository checks and automation (read `cargo x --help` and `<subcommand> --help` for options):

```shell
cargo x build              # Compile all workspace packages and targets with all features
cargo x test               # Run workspace tests with all features enabled (--no-capture supported)
cargo x lint               # Run nightly clippy, rustfmt, taplo, typos, and hawkeye checks
cargo x lint --fix         # Automatically apply clippy, rustfmt, taplo, and hawkeye fixes
```

`cargo x lint` requires a `nightly` toolchain with `rustfmt` and `clippy`. CI also verifies both `stable` and the workspace MSRV (`rust-version` in `Cargo.toml`) across Linux, macOS, and Windows alongside runnable examples in `examples/`.

## Rust Style

- **Modules and imports**: Order module headers as `//!` docs, `use` imports (`group_imports = "StdExternalCrate"`, `imports_granularity = "Item"`), out-of-line `mod` declarations, and items. Use `<module>/mod.rs` for multi-file modules, and do not split cohesive files prematurely. Group `mod` declarations with blank lines between `pub mod`, private `mod` with immediate `pub use self::...;` re-exports, and implementation-only private `mod` (`#[cfg(test)]` last), sorted alphabetically within each group; re-export subsystem interfaces from `mod.rs` and attach `#[cfg]` gates at module boundaries when possible. Import intra-crate symbols from `crate::...` (avoiding `super::...` and private `self::...`, reserving `use super::*;` for `#[cfg(test)]` modules), and never write inline `crate::...` or `super::...` paths in expressions or types. Prefer module-header `use` imports, but keep `use` local to a `#[cfg]`-gated block or function (or write an external or standard-library path such as `std::error::Error` or `serde::Serialize` inline) when doing so avoids duplicating `#[cfg]` gates, name collisions, or single-site import clutter. Preserve original names with meaningful module or crate qualification (such as `io::Error`, `fs::File`, or `process::Command`), and avoid `use ... as ...` aliases except `use ...::Trait as _;` for method-only traits.
- **Visibility**: Use three visibility tiers: private by default, plain `pub` inside private ancestor module trees, and `pub(crate)` for internal fields, constructors, or helpers reachable through `pub mod` or `pub use` re-exported types. Avoid `pub(super)` and `pub(in ...)`, and never widen visibility solely for cross-module tests.
- **Types, traits, and lints**: Name items by concrete role, keep a type and its primary `impl` together (public API before private helpers), and reserve `len` and `is_empty` for collection or slice-like views. Use `new` (with `Default` when parameterless and `const fn` when trivial), `with_*` for alternative required inputs, and `*Builder` for multi-option construction. Do not prefix struct fields with `_` to silence warnings; keep descriptive names and add `#[expect(dead_code, reason = "...")]` on unread ownership or guard fields. Omit `PartialEq` and `Eq` when field equality would expose internal layout or history, avoid unnecessary generic bounds on custom `Debug` impls, reserve `#[must_use = "..."]` for lazy values, builders, or scope guards whose drop or omission loses behavior, and use `#[expect(..., reason = "...")]` instead of `#[allow(...)]`.
- **Control flow and errors**: Keep abstractions at signatures, cohesive types, and module boundaries; inside functions, prefer linear control flow with guard returns, explicit `match` or `if let`, and explicit `for` loops for multi-step or fallible logic over premature helpers or uncontracted defensive glue. Return `Result<_, Error>` (keeping failing mutations atomic when practical) for recoverable runtime failures without panicking on operational I/O or formatting inputs (see [DESIGN.md](DESIGN.md#error-handling-and-operational-safety)), use empty values or `Option` for absent queries, reserve fail-fast panics for broken internal invariants, and preserve structured error context rather than parsing formatted strings.

## Documentation

- Keep each Markdown prose paragraph and list item on one source line, and format Markdown tables so their columns and delimiters align in the source.
- Document caller-observable contracts in `///` comments without leaking implementation details. Use noun phrases for types, third-person present-tense verbs for operations, and public API labels on intra-doc links (such as `` [`Layout`][logforth_core::layout::Layout] ``).
- Include `# Errors`, `# Panics`, `# Safety`, and trailing `# Examples` (with visible imports) only when applicable, explaining simple return values in the summary or body.
- Reserve `//` comments for non-obvious invariants, `// SAFETY:` preconditions, or external quirks rather than restating code, and preserve existing provenance and license notices.

## Testing

- Exercise observable contracts through public APIs in crate `tests/` targets, and keep `#[cfg(test)]` unit tests next to the implementation strictly for private invariants that black-box tests cannot reach.
- Every test must guard a realistic regression without mirroring literals, trivial control flow, static type guarantees, or non-contract `Debug` wording.
- Avoid test-only production hooks, and coordinate concurrent tests deterministically instead of sleeping.

## Changelog

- Update `CHANGELOG.md` in the same pull request for significant user-visible changes by comparing the final behavior with the latest release tag rather than recording the sequence of commits that produced it.
- Before adding a bug-fix entry, verify from the latest release tag that the faulty behavior was shipped. If the affected API or behavior is itself unreleased, describe only its final contract in the relevant feature entry and omit the development-only correction.
- Include public API migrations, new capabilities, correctness or compatibility changes, and meaningful performance improvements. Exclude tests, internal refactors, documentation, CI, tooling, and dependency maintenance unless they change supported or observable behavior relative to the latest release.
- Keep the permanent `## Unreleased` section at the top. Group entries under `### Breaking changes`, `### New features`, `### Improvements`, and `### Bug fixes` in that order, adding only categories that contain entries.
- Write one bullet for each coherent behavior from the user's perspective: name the affected public API or crate and its observable result, and give required migration guidance for breaking changes without pull request numbers, issue numbers, or discarded intermediate designs.
- During release preparation, insert `## vX.Y.Z` without a date immediately below `## Unreleased` and move the accumulated entries into it. Add the actual UTC release date in `YYYY-MM-DD` format after publication; do not guess it in advance or remove `## Unreleased`.

## Pull Requests

Format pull request titles according to `.github/semantic.yml` and keep the description concise. Use a `Summary` section for routine changes and add `Design Notes` only when the design needs explanation.
