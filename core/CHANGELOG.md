# CHANGELOG

All notable changes to the `logforth-core` crate will be documented in this file.

## Unreleased

### Breaking changes

* `default_logger` now always return a `&'static Logger`. If it is not set by the user, it will return a no-op logger.

## [0.1.0] 2025-09-28

This is the initial release.
