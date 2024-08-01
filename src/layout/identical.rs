use crate::layout::{Layout, LayoutImpl};

#[derive(Debug, Default, Clone, Copy)]
pub struct IdenticalLayout;

impl Layout for IdenticalLayout {
    fn format_record(&self, record: &log::Record) -> anyhow::Result<log::Record> {
        Ok(record.clone())
    }
}

impl From<IdenticalLayout> for LayoutImpl {
    fn from(layout: IdenticalLayout) -> Self {
        LayoutImpl::Identical(layout)
    }
}
