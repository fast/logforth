# File Appender

File appender for Logforth supporting single-file output and direct-write time- and size-based segment rotation.

* Writes directly to append-mode segment files without renaming active files on rollover.
* Supports time-based (`Minutely`, `Hourly`, `Daily` with configurable `TimeZone`), size-based (`NonZeroU64`), and combined rotation with global segment count retention (`max_log_files`).
* Delegates asynchronous buffering to `logforth-append-async` rather than bundling a dedicated worker thread.

Design references:

* Apache Log4j 2 [`DirectWriteRolloverStrategy`](https://logging.apache.org/log4j/2.x/manual/appenders/rolling-file.html)
* Logback [`SizeAndTimeBasedRollingPolicy`](https://logback.qos.ch/manual/appenders.html#SizeAndTimeBasedRollingPolicy)
* Go [`lumberjack`](https://github.com/natefinch/lumberjack)
