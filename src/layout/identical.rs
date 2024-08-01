
use crate::layout::Layout;
use crate::layout::LayoutImpl;

#[derive(Debug, Default, Clone, Copy)]
pub struct Identical;

impl Layout for Identical {
    fn format<F>(&self, record: &log::Record, f: F) -> anyhow::Result<()>
    where
        F: Fn(&log::Record) -> anyhow::Result<()>,
    {
        f(record)
    }
}

impl From<Identical> for LayoutImpl {
    fn from(layout: Identical) -> Self {
        LayoutImpl::Identical(layout)
    }
}
