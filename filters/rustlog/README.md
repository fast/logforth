# Logforth `RUST_LOG` Filter

`logforth-filter-rustlog` provides [`RustLogFilter`](https://docs.rs/logforth-filter-rustlog/latest/logforth_filter_rustlog/struct.RustLogFilter.html) and [`RustLogFilterBuilder`](https://docs.rs/logforth-filter-rustlog/latest/logforth_filter_rustlog/struct.RustLogFilterBuilder.html), implementing `logforth_core::Filter` with `RUST_LOG`-style prefix and fallback directives.

Most applications can enable this crate through the [`logforth`](https://crates.io/crates/logforth) facade crate with the `filter-rustlog` feature (included in `starter-log`). Depend on `logforth-filter-rustlog` directly when composing split `logforth-*` crates onto `logforth-core`.

## Quick start

```shell
cargo add logforth-core logforth-filter-rustlog
```

```rust
use logforth_core::LoggerBuilder;
use logforth_core::append::Stderr;
use logforth_core::record::Level;
use logforth_core::record::LevelFilter;
use logforth_filter_rustlog::RustLogFilterBuilder;

fn build_logger() -> Result<logforth_core::Logger, logforth_core::Error> {
    let builder = RustLogFilterBuilder::from_default_env_or("info,my_app::db=debug")?;
    let filter = builder
        .prefix("hyper", LevelFilter::MoreSevereEqual(Level::Warn))
        .build();

    Ok(LoggerBuilder::new()
        .dispatch(|d| d.filter(filter).append(Stderr::new()))
        .build())
}
```

## Directive syntax

Specifications consist of comma-separated directives evaluated case-insensitively across all 24 Logforth severity levels (`trace`, `trace2`..`trace4`, `debug`..`debug4`, `info`..`info4`, `warn`..`warn4`, `error`..`error4`, `fatal`..`fatal4`) plus the pseudo-levels `off` and `all`:

| Directive        | Meaning                                                                             |
| :--------------- | :---------------------------------------------------------------------------------- |
| `info` / `=info` | Sets the unmatched fallback threshold to `info` and more severe levels.             |
| `off` / `all`    | Disables or enables all levels as the unmatched fallback (`=` is alias for `all`).  |
| `my_crate`       | Enables all levels (`all`) for targets starting with `my_crate`.                    |
| `my_crate=`      | Equivalent to `my_crate` (enables all levels for targets starting with `my_crate`). |
| `my_crate=warn`  | Enables `warn` and more severe levels for targets starting with `my_crate`.         |
| `my_crate=off`   | Disables all levels for targets starting with `my_crate`.                           |

- **Longest prefix wins**: When multiple prefix directives match a record, the longest matching prefix determines the threshold.
- **Target resolution**: Prefix directives match an explicit `"logforth.target"` string key-value entry on the record when present (for example, custom targets forwarded by `logforth-bridge-log`), falling back to `Record::module_path`.
- **Fail-fast validation**: Malformed directives and non-UTF-8 environment variables return `logforth_core::Error` immediately instead of printing warnings to standard error.

## Acknowledgments

Directive syntax and prefix matching are adapted from [`env_filter`](https://crates.io/crates/env_filter), tailored to Logforth's 24-level model, `FilterResult` pipeline, and structured target overrides without regex payload filtering.

## Documentation

- [Rust API documentation](https://docs.rs/logforth-filter-rustlog)
- [Logforth facade crate](https://docs.rs/logforth)

## License

Licensed under the Apache License, Version 2.0.
