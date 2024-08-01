use log::{Record, RecordBuilder};

pub fn make_record_with_args(args: std::fmt::Arguments, record: &Record) -> Record {
    RecordBuilder::new()
        .args(args)
        .metadata(record.metadata().clone())
        .level(record.level())
        .target(record.target())
        .module_path(record.module_path())
        .file(record.file())
        .line(record.line())
        .build()
}
