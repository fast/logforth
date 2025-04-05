// Copyright 2024 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::borrow::Cow;
use std::io;
use std::io::Write;
use std::os::unix::net::UnixDatagram;

use log::Level;
use log::Record;

use crate::diagnostic::Visitor;
use crate::Append;
use crate::Diagnostic;

mod field;
#[cfg(target_os = "linux")]
mod memfd;

const JOURNALD_PATH: &str = "/run/systemd/journal/socket";

fn current_exe_identifier() -> Option<String> {
    let executable = std::env::current_exe().ok()?;
    Some(executable.file_name()?.to_string_lossy().into_owned())
}

/// A systemd journal appender.
///
/// ## Journal access
///
/// ## Standard fields
///
/// The journald appender always sets the following standard [journal fields]:
///
/// - `PRIORITY`: The log level mapped to a priority (see below).
/// - `MESSAGE`: The formatted log message (see [`Record::args()`]).
/// - `SYSLOG_PID`: The PID of the running process (see [`std::process::id()`]).
/// - `CODE_FILE`: The filename the log message originates from (see [`Record::file()`], only if
///   present).
/// - `CODE_LINE`: The line number the log message originates from (see [`Record::line()`], only if
///   present).
///
/// It also sets `SYSLOG_IDENTIFIER` if non-empty (see [`Journald::with_syslog_identifier`]).
///
/// Additionally, it also adds the following non-standard fields:
///
/// - `TARGET`: The target of the log record (see [`Record::target()`]).
/// - `CODE_MODULE`: The module path of the log record (see [`Record::module_path()`], only if
///   present).
///
/// [journal fields]: https://www.freedesktop.org/software/systemd/man/systemd.journal-fields.html
///
/// ## Log levels and Priorities
///
/// [`Level`] gets mapped to journal (syslog) priorities as follows:
///
/// - [`Level::Error`] → `3` (err)
/// - [`Level::Warn`] → `4` (warning)
/// - [`Level::Info`] → `5` (notice)
/// - [`Level::Debug`] → `6` (info)
/// - [`Level::Trace`] → `7` (debug)
///
/// Higher priorities (crit, alert, and emerg) are not used.
///
/// ## Custom fields and structured record fields
///
/// In addition to these fields the appender also adds all structures key-values
/// (see [`Record::key_values`]) from each log record as journal fields,
/// and also supports global extra fields via [`Journald::with_extra_fields`].
///
/// Journald allows only ASCII uppercase letters, ASCII digits, and the
/// underscore in field names, and limits field names to 64 bytes.  See
/// [`journal_field_valid`][jfv] for the precise validation rules.
///
/// This appender mangles the keys of additional key-values on records and names
/// of custom fields according to the following rules, to turn them into valid
/// journal fields:
///
/// - If the key is entirely empty, use `EMPTY`.
/// - Transform the entire value to ASCII uppercase.
/// - Replace all invalid characters with underscore.
/// - If the key starts with an underscore or digit, which is not permitted, prepend `ESCAPED_`.
/// - Cap the result to 64 bytes.
///
/// [jfv]: https://github.com/systemd/systemd/blob/v256.7/src/libsystemd/sd-journal/journal-file.c#L1703
///
/// # Errors
///
/// The appender tries to connect to journald when constructed, to provide early
/// on feedback if journald is not available (e.g. in containers where the
/// journald socket is not mounted into the container).
#[derive(Debug)]
pub struct Journald {
    /// The datagram socket to send messages to journald.
    socket: UnixDatagram,
    /// Preformatted extra fields to be appended to every log message.
    extra_fields: Vec<u8>,
    /// The syslog identifier.
    syslog_identifier: String,
}

impl Journald {
    /// Construct a journald appender
    ///
    /// Fails if the journald socket couldn't be opened.
    pub fn new() -> io::Result<Self> {
        let socket = UnixDatagram::unbound()?;
        let sub = Self {
            socket,
            extra_fields: Vec::new(),
            syslog_identifier: current_exe_identifier().unwrap_or_default(),
        };
        // Check that we can talk to journald, by sending empty payload which journald discards.
        // However, if the socket didn't exist or if none listened we'd get an error here.
        sub.send_payload(&[])?;
        Ok(sub)
    }

    /// Add an extra field to be added to every log entry.
    ///
    /// `name` is the name of a custom field, and `value` its value. Fields are
    /// appended to every log entry, in order they were added to the appender.
    ///
    /// ## Restrictions on field names
    ///
    /// `name` should be a valid journal file name, i.e. it must only contain
    /// ASCII uppercase alphanumeric characters and the underscore, and must
    /// start with an ASCII uppercase letter.
    ///
    /// Invalid keys in `extra_fields` are escaped according to the rules
    /// documented in [`Journald`].
    ///
    /// It is not recommended that `name` is any of the standard fields already
    /// added by this appender (see [`Journald`]); though journald supports
    /// multiple values for a field, journald clients may not handle unexpected
    /// multi-value fields properly and perhaps only show the first value.
    /// Specifically, even `journalctl` will only show the first `MESSAGE` value
    /// of journal entries.
    ///
    /// ## Restrictions on values
    ///
    /// There are no restrictions on the value.
    pub fn with_extra_field<K: AsRef<str>, V: AsRef<[u8]>>(mut self, name: K, value: V) -> Self {
        field::put_field_bytes(
            &mut self.extra_fields,
            field::FieldName::WriteEscaped(name.as_ref()),
            value.as_ref(),
        );
        self
    }

    /// Add extra fields to be added to every log entry.
    ///
    /// See [`Self::with_extra_field`] for details.
    pub fn with_extra_fields<I, K, V>(mut self, extra_fields: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<[u8]>,
    {
        for (name, value) in extra_fields {
            field::put_field_bytes(
                &mut self.extra_fields,
                field::FieldName::WriteEscaped(name.as_ref()),
                value.as_ref(),
            );
        }
        self
    }

    /// Sets the syslog identifier for this appender.
    ///
    /// The syslog identifier comes from the classic syslog interface (`openlog()`
    /// and `syslog()`) and tags log entries with a given identifier.
    /// Systemd exposes it in the `SYSLOG_IDENTIFIER` journal field, and allows
    /// filtering log messages by syslog identifier with `journalctl -t`.
    /// Unlike the unit (`journalctl -u`) this field is not trusted, i.e. applications
    /// can set it freely, and use it e.g. to further categorize log entries emitted under
    /// the same systemd unit or in the same process.  It also allows to filter for log
    /// entries of processes not started in their own unit.
    ///
    /// See [Journal Fields](https://www.freedesktop.org/software/systemd/man/systemd.journal-fields.html)
    /// and [journalctl](https://www.freedesktop.org/software/systemd/man/journalctl.html)
    /// for more information.
    ///
    /// Defaults to the file name of the executable of the current process, if any.
    pub fn with_syslog_identifier(mut self, identifier: String) -> Self {
        self.syslog_identifier = identifier;
        self
    }

    /// Returns the syslog identifier in use.
    pub fn syslog_identifier(&self) -> &str {
        &self.syslog_identifier
    }

    fn send_payload(&self, payload: &[u8]) -> io::Result<usize> {
        self.socket
            .send_to(payload, JOURNALD_PATH)
            .or_else(|error| {
                if Some(libc::EMSGSIZE) == error.raw_os_error() {
                    self.send_large_payload(payload)
                } else {
                    Err(error)
                }
            })
    }

    #[cfg(all(unix, not(target_os = "linux")))]
    fn send_large_payload(&self, _payload: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Large payloads not supported on non-Linux OS",
        ))
    }

    /// Send large payloads to journald via a memfd.
    #[cfg(target_os = "linux")]
    fn send_large_payload(&self, payload: &[u8]) -> io::Result<usize> {
        memfd::send_large_payload(&self.socket, payload)
    }
}

struct WriteKeyValues<'a>(&'a mut Vec<u8>);

impl<'kvs> log::kv::VisitSource<'kvs> for WriteKeyValues<'_> {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        let key = key.as_str();
        field::put_field_length_encoded(self.0, field::FieldName::WriteEscaped(key), value);
        Ok(())
    }
}

impl Visitor for WriteKeyValues<'_> {
    fn visit(&mut self, key: Cow<str>, value: Cow<str>) -> anyhow::Result<()> {
        let key = key.as_ref();
        let value = value.as_bytes();
        field::put_field_length_encoded(self.0, field::FieldName::WriteEscaped(key), value);
        Ok(())
    }
}

impl Append for Journald {
    /// Extract all fields (standard and custom) from `record`, append all `extra_fields` given
    /// to this appender, and send the result to journald.
    fn append(&self, record: &Record, diagnostics: &[Box<dyn Diagnostic>]) -> anyhow::Result<()> {
        use field::*;

        let mut buffer = vec![];

        // Write standard fields. Numeric fields can't contain new lines so we
        // write them directly, everything else goes through the put functions
        // for property mangling and length-encoding
        let priority = match record.level() {
            Level::Error => b"3",
            Level::Warn => b"4",
            Level::Info => b"5",
            Level::Debug => b"6",
            Level::Trace => b"7",
        };

        put_field_bytes(&mut buffer, FieldName::WellFormed("PRIORITY"), priority);
        put_field_length_encoded(&mut buffer, FieldName::WellFormed("MESSAGE"), record.args());
        // Syslog compatibility fields
        writeln!(&mut buffer, "SYSLOG_PID={}", std::process::id())?;
        if !self.syslog_identifier.is_empty() {
            put_field_bytes(
                &mut buffer,
                FieldName::WellFormed("SYSLOG_IDENTIFIER"),
                self.syslog_identifier.as_bytes(),
            );
        }
        if let Some(file) = record.file() {
            put_field_bytes(
                &mut buffer,
                FieldName::WellFormed("CODE_FILE"),
                file.as_bytes(),
            );
        }
        if let Some(module) = record.module_path() {
            put_field_bytes(
                &mut buffer,
                FieldName::WellFormed("CODE_MODULE"),
                module.as_bytes(),
            );
        }
        if let Some(line) = record.line() {
            writeln!(&mut buffer, "CODE_LINE={}", line)?;
        }
        put_field_bytes(
            &mut buffer,
            FieldName::WellFormed("TARGET"),
            record.target().as_bytes(),
        );
        // Put all structured values of the record
        let mut visitor = WriteKeyValues(&mut buffer);
        record.key_values().visit(&mut visitor)?;
        for d in diagnostics {
            d.visit(&mut visitor)?;
        }
        // Put all extra fields of the appender
        buffer.extend_from_slice(&self.extra_fields);
        self.send_payload(&buffer)?;
        Ok(())
    }
}
