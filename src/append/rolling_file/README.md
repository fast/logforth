# File Appender

This appender is originally a fork of [tracing-appender](https://crates.io/crates/tracing-appender).

Later, we rewrite it completely to support:

* Rolling files based on file size and/or time.
* Drop non-blocking glue in favor of a dedicated async appender combinator.
* Different log file naming strategies.

Design reference:

* https://logback.qos.ch/manual/appenders.html#SizeAndTimeBasedRollingPolicy
* https://logging.apache.org/log4j/2.x/manual/appenders/rolling-file.html
