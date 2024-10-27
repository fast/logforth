use crate::filter::FilterResult;
use crate::{Append, Filter, Logger};
use log::{Metadata, Record};

/// A grouped set of appenders and filters.
///
/// The [`Logger`] facade dispatches log records to one or more [`Dispatch`] instances.
/// Each [`Dispatch`] instance contains a set of filters and appenders.
///
/// `filters` are used to determine whether a log record should be passed to the appenders.
/// `appends` are used to write log records to a destination.
#[derive(Debug)]
pub struct Dispatch<const APPEND: bool = true> {
    filters: Vec<Filter>,
    appends: Vec<Box<dyn Append>>,
}

impl Default for Dispatch<false> {
    fn default() -> Dispatch<false> {
        Self::new()
    }
}

impl Dispatch<false> {
    /// Create a new incomplete [`Dispatch`] instance.
    ///
    /// At least one append must be added to the [`Dispatch`] before it can be used.
    pub fn new() -> Dispatch<false> {
        Self {
            filters: vec![],
            appends: vec![],
        }
    }

    /// Add a [`Filter`] to the [`Dispatch`].
    pub fn filter(mut self, filter: impl Into<Filter>) -> Dispatch<false> {
        self.filters.push(filter.into());
        self
    }
}

impl<const APPEND: bool> Dispatch<APPEND> {
    /// Add an [`Append`] to the [`Dispatch`].
    pub fn append(mut self, append: impl Append) -> Dispatch<true> {
        self.appends.push(Box::new(append));

        Dispatch {
            filters: self.filters,
            appends: self.appends,
        }
    }
}

impl Dispatch {
    fn enabled(&self, metadata: &Metadata) -> bool {
        for filter in &self.filters {
            match filter.enabled(metadata) {
                FilterResult::Reject => return false,
                FilterResult::Accept => return true,
                FilterResult::Neutral => {}
            }
        }

        true
    }

    fn log(&self, record: &Record) -> anyhow::Result<()> {
        for filter in &self.filters {
            match filter.matches(record) {
                FilterResult::Reject => return Ok(()),
                FilterResult::Accept => break,
                FilterResult::Neutral => {}
            }
        }

        for append in &self.appends {
            append.append(record)?;
        }
        Ok(())
    }

    fn flush(&self) {
        for append in &self.appends {
            append.flush();
        }
    }
}
