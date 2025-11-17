# RUST_LOG Environment Variable Filter

This filter is derived by [env_filter](https://crates.io/crates/env_filter), with significant modifications to suit our needs:

1. Logforth needs not the original `FilterLog` struct.
2. Logforth would use its own `Level`, `LevelFilter`, `FilterCriteria`, and `Record` types.
3. The regex based global filter is discarded. Filtering by targets should be sufficient.
4. Interfaces and methods are refactored to be more ergonomic.
