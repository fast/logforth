use log::Record;

use crate::layout::Layout;
use crate::layout::LayoutImpl;

#[derive(Debug, Default, Clone, Copy)]
pub struct Identical;

impl Layout for Identical {
    fn format_record(&self, record: Record) -> anyhow::Result<Record> {
        Ok(record)
    }
}

impl From<Identical> for LayoutImpl {
    fn from(layout: Identical) -> Self {
        LayoutImpl::Identical(layout)
    }
}
