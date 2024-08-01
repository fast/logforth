use std::fmt::Arguments;

use crate::layout::Layout;

#[derive(Debug, Default, Clone, Copy)]
pub struct Identical;

impl Identical {
    pub fn format<F>(&self, record: &log::Record, f: &F) -> anyhow::Result<()>
    where
        F: Fn(Arguments) -> anyhow::Result<()>,
    {
        f(*record.args())
    }
}

impl From<Identical> for Layout {
    fn from(layout: Identical) -> Self {
        Layout::Identical(layout)
    }
}
