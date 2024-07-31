use std::fmt::Arguments;

use log::Record;

/// Convert records to final format arguments.
#[enum_dispatch::enum_dispatch]
pub trait Format: Send + Sync {
    fn format<'a>(&'a self, record: &'a Record) -> Arguments;
}

#[enum_dispatch::enum_dispatch(Format)]
pub enum FormatImpl {
    BoxDyn(Box<dyn Format>),
}

impl Format for Box<dyn Format> {
    fn format<'a>(&'a self, record: &'a Record) -> Arguments {
        (**self).format(record)
    }
}

impl<T: for<'a> Fn(&'a Record<'a>) -> Arguments<'a> + Send + Sync + 'static> Format for T {
    fn format<'a>(&'a self, record: &'a Record) -> Arguments {
        (*self)(record)
    }
}

impl<T: for<'a> Fn(&'a Record<'a>) -> Arguments<'a> + Send + Sync + 'static> From<T>
    for FormatImpl
{
    fn from(t: T) -> Self {
        FormatImpl::BoxDyn(Box::new(t))
    }
}
