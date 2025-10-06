# Asynchronous Appender

This appender is a remix of [spdlog-rs's AsyncPoolSink](https://docs.rs/spdlog-rs/*/spdlog/sink/struct.AsyncPoolSink.html), with several modifications to fit this crate's need:

* Instead of a thread pool, it uses a single background thread to drain the log queue.
