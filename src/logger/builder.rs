use super::log_impl::Dispatch;
use super::log_impl::Logger;
use crate::Append;
use crate::Filter;

/// Create a new empty [builder][Builder].
///
/// The builder must be configured before initializing the global logger. At least one dispatch
/// should be added:
///
/// ```rust
/// use log::LevelFilter;
/// use logforth::append;
///
/// logforth::builder()
///     // .finish()  CANNOT COMPILE: a staging dispatch without Append
///     .filter(LevelFilter::Info)
///     .append(append::Stdout::default())
///     .finish()
///     .unwrap();
/// ```
///
/// Multiple dispatches can be added:
///
/// ```rust
/// use log::LevelFilter;
/// use logforth::append;
///
/// logforth::builder()
///     .filter(LevelFilter::Info)
///     .append(append::Stdout::default())
///     .dispatch() // finish the current dispatch and start a new staging dispatch with no Append and Filter configured
///     .filter(LevelFilter::Debug)
///     .append(append::Stderr::default())
///     .finish()
///     .unwrap();
/// ```
pub fn builder() -> Builder<false> {
    Builder::default()
}

/// A builder for configuring the logger. See also [`builder`] for a fluent API.
///
/// * `READY=false`: The initialized state. You can configure [`Filter`]s and [`Append`]s for the
///   current staging dispatch. Once at least one append is configured, the builder transit to
///   `READY=true`.
/// * `READY=true`: The builder can be [finished][Builder::finish] to set up the global logger. Or,
///   you can start a new staging dispatch by calling [dispatch][Builder::dispatch].
///
/// ## Examples
///
/// Create a new builder and configure filters and appends:
///
/// ```rust
/// use log::LevelFilter;
/// use logforth::append;
///
/// logforth::Builder::new()
///     .filter(LevelFilter::Info)
///     .append(append::Stdout::default())
///     .finish()
///     .unwrap();
/// ```
// TODO(tisonkun): consider use an enum as const generic param once `adt_const_params` stabilized.
//  @see https://doc.rust-lang.org/beta/unstable-book/language-features/adt-const-params.html
#[must_use = "call `dispatch` to add a dispatch to the logger and `finish` to set the global logger"]
#[derive(Debug)]
pub struct Builder<const READY: bool = true> {
    // for current dispatch
    filters: Vec<Filter>,
    appends: Vec<Box<dyn Append>>,

    // stashed dispatches
    dispatches: Vec<Dispatch>,
}

impl Default for Builder<false> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const READY: bool> Builder<READY> {
    /// Add an [`Append`] to the under constructing `Dispatch`.
    pub fn append(mut self, append: impl Append) -> Builder<true> {
        self.appends.push(Box::new(append));

        Builder {
            filters: self.filters,
            appends: self.appends,
            dispatches: self.dispatches,
        }
    }
}

impl Builder<false> {
    /// Create a new empty [`Builder`].
    pub fn new() -> Self {
        Self {
            filters: vec![],
            appends: vec![],
            dispatches: vec![],
        }
    }

    /// Add a [`Filter`] to the under constructing `Dispatch`.
    pub fn filter(mut self, filter: impl Into<Filter>) -> Builder<false> {
        self.filters.push(filter.into());
        self
    }
}

impl Builder<true> {
    /// Construct a new `Dispatch` with the configured [`Filter`]s and [`Append`]s.
    pub fn dispatch(mut self) -> Builder<false> {
        let dispatch = Dispatch::new(self.filters, self.appends);
        self.dispatches.push(dispatch);

        Builder {
            filters: vec![],
            appends: vec![],
            dispatches: self.dispatches,
        }
    }

    /// Set up the global logger with all the dispatches configured.
    ///
    /// # Errors
    ///
    /// An error is returned if the global logger has already been set.
    pub fn finish(self) -> Result<(), log::SetLoggerError> {
        debug_assert!(
            !self.dispatches.is_empty(),
            "Generic type should ensure dispatches is not empty"
        );

        let logger = Logger::new(self.dispatches);
        log::set_boxed_logger(Box::new(logger))?;
        Ok(())
    }
}
