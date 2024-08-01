use log::Record;

use crate::layout::Layout;
use crate::layout::LayoutImpl;

#[derive(Debug, Default, Clone, Copy)]
pub struct Identical;

impl Layout for Identical {
    fn format_record<'a>(&'_ self, record: &'a Record<'a>) -> anyhow::Result<Record<'a>> {
        Ok(record.clone())
    }
}

impl From<Identical> for LayoutImpl {
    fn from(layout: Identical) -> Self {
        LayoutImpl::Identical(layout)
    }
}
