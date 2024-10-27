use log::{LevelFilter, Metadata, Record};
use crate::logger::dispatch::Dispatch;

/// A logger facade that dispatches log records to one or more [`Dispatch`] instances.
///
/// This struct implements [`log::Log`] to bridge Logforth's logging implementations
/// with the [`log`] crate.
#[derive(Debug)]
pub struct Logger {
    dispatches: Vec<Dispatch>,
    max_level: LevelFilter,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Logger {
    /// Create a new [`Logger`] instance.
    #[must_use = "call `dispatch` to add a dispatch to the logger and `apply` to set the global logger"]
    pub fn new() -> Logger {
        Self {
            dispatches: vec![],
            max_level: LevelFilter::Trace,
        }
    }

    /// Set the global maximum log level.
    ///
    /// This will be passed to [`log::set_max_level`] on [`Logger::apply`].
    #[must_use = "call `apply` to set the global logger"]
    pub fn max_level(mut self, max_level: LevelFilter) -> Self {
        self.max_level = max_level;
        self
    }

    /// Add a [`Dispatch`] to the [`Logger`].
    #[must_use = "call `apply` to set the global logger"]
    pub fn dispatch(mut self, dispatch: Dispatch) -> Self {
        self.dispatches.push(dispatch);
        self
    }

    /// Set up the global logger with the [`Logger`] instance.
    ///
    /// # Errors
    ///
    /// An error is returned if the global logger has already been set.
    pub fn apply(self) -> Result<(), log::SetLoggerError> {
        let max_level = self.max_level;
        log::set_boxed_logger(Box::new(self))?;
        log::set_max_level(max_level);
        Ok(())
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.dispatches
            .iter()
            .any(|dispatch| dispatch.enabled(metadata))
    }

    fn log(&self, record: &Record) {
        for dispatch in &self.dispatches {
            if let Err(err) = dispatch.log(record) {
                handle_error(record, err);
            }
        }
    }

    fn flush(&self) {
        for dispatch in &self.dispatches {
            dispatch.flush();
        }
    }
}

fn handle_error(record: &Record, error: anyhow::Error) {
    let Err(fallback_error) = write!(
        std::io::stderr(),
        r###"
Error perform logging.
    Attempted to log: {args}
    Record: {record:?}
    Error: {error}
"###,
        args = record.args(),
        record = record,
        error = error,
    ) else {
        return;
    };

    panic!(
        r###"
Error performing stderr logging after error occurred during regular logging.
    Attempted to log: {args}
    Record: {record:?}
    Error: {error}
    Fallback error: {fallback_error}
"###,
        args = record.args(),
        record = record,
        error = error,
        fallback_error = fallback_error,
    );
}
