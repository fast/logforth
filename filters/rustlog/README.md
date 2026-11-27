# Logforth RUST_LOG Filter

This filter is derived by [env_filter](https://crates.io/crates/env_filter), with significant modifications to suit our needs:

1. Logforth needs not the original `FilterLog` struct.
2. Logforth uses its own `Level`, `LevelFilter`, `FilterResult`, and `Record` types.
3. The regex based global filter is discarded. Prefix matching on module paths (or explicit target overrides) is sufficient.
4. Interfaces and methods are refactored to be more ergonomic.
