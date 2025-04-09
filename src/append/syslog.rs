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

//! Appender for writing log records to syslog.
//!
//! # Examples
//!
//!```rust, no_run
//! use logforth::append::syslog;
//! use logforth::append::syslog::Syslog;
//! use logforth::append::syslog::SyslogBuilder;
//!
//! let (append, _guard) = SyslogBuilder::tcp_well_known().unwrap().build();
//!
//! logforth::builder()
//!     .dispatch(|d| d.filter(log::LevelFilter::Trace).append(append))
//!     .apply();
//!
//! log::info!("This log will be written to syslog.");
//! ```

use std::io;
use std::time::Duration;

use fasyslog::format::SyslogContext;
use fasyslog::sender::SyslogSender;
use fasyslog::SDElement;
use log::Record;

use crate::non_blocking::NonBlocking;
use crate::non_blocking::NonBlockingBuilder;
use crate::non_blocking::Writer;
use crate::Append;
use crate::Diagnostic;
use crate::DropGuard;
use crate::Layout;

pub extern crate fasyslog;

/// The format of the syslog message.
#[derive(Debug, Copy, Clone)]
pub enum SyslogFormat {
    /// [RFC 3614] (BSD syslog Protocol)
    ///
    /// [RFC 3164]: https://datatracker.ietf.org/doc/html/rfc3164
    RFC3164,
    /// [RFC 5424] (The Syslog Protocol)
    ///
    /// [RFC 5424]: https://datatracker.ietf.org/doc/html/rfc5424
    RFC5424,
}

/// A builder to configure and create an [`Syslog`] appender.
#[derive(Debug)]
pub struct SyslogBuilder {
    sender: SyslogSender,
    formatter: SyslogFormatter,

    // non-blocking options
    thread_name: String,
    buffered_lines_limit: Option<usize>,
    shutdown_timeout: Option<Duration>,
}

impl SyslogBuilder {
    /// Create a new builder.
    pub fn new(sender: SyslogSender) -> Self {
        Self {
            sender,
            formatter: SyslogFormatter {
                format: SyslogFormat::RFC3164,
                context: SyslogContext::default(),
                layout: None,
            },
            thread_name: "logforth-syslog".to_string(),
            buffered_lines_limit: None,
            shutdown_timeout: None,
        }
    }

    /// Build the [`Syslog`] appender.
    pub fn build(self) -> (Syslog, DropGuard) {
        let SyslogBuilder {
            sender,
            formatter,
            thread_name,
            buffered_lines_limit,
            shutdown_timeout,
        } = self;
        let (non_blocking, guard) = NonBlockingBuilder::new(thread_name, SyslogWriter { sender })
            .buffered_lines_limit(buffered_lines_limit)
            .shutdown_timeout(shutdown_timeout)
            .build();
        (Syslog::new(non_blocking, formatter), Box::new(guard))
    }

    /// Set the format of the [`Syslog`] appender.
    pub fn format(mut self, format: SyslogFormat) -> Self {
        self.formatter.format = format;
        self
    }

    /// Set the context of the [`Syslog`] appender.
    pub fn context(mut self, context: SyslogContext) -> Self {
        self.formatter.context = context;
        self
    }

    /// Set the layout of the [`Syslog`] appender.
    ///
    /// Default to `None`, the message will construct with only [`Record::args`].
    pub fn layout(mut self, layout: impl Into<Box<dyn Layout>>) -> Self {
        self.formatter.layout = Some(layout.into());
        self
    }

    /// Sets the buffer size of pending messages.
    pub fn buffered_lines_limit(mut self, buffered_lines_limit: Option<usize>) -> Self {
        self.buffered_lines_limit = buffered_lines_limit;
        self
    }

    /// Sets the shutdown timeout before the worker guard dropped.
    pub fn shutdown_timeout(mut self, shutdown_timeout: Option<Duration>) -> Self {
        self.shutdown_timeout = shutdown_timeout;
        self
    }

    /// Sets the thread name for the background sender thread.
    pub fn thread_name(mut self, thread_name: impl Into<String>) -> Self {
        self.thread_name = thread_name.into();
        self
    }

    /// Create a new syslog writer that sends messages to the well-known TCP port (514).
    pub fn tcp_well_known() -> io::Result<SyslogBuilder> {
        fasyslog::sender::tcp_well_known()
            .map(SyslogSender::Tcp)
            .map(Self::new)
    }

    /// Create a new syslog writer that sends messages to the given TCP address.
    pub fn tcp<A: std::net::ToSocketAddrs>(addr: A) -> io::Result<SyslogBuilder> {
        fasyslog::sender::tcp(addr)
            .map(SyslogSender::Tcp)
            .map(Self::new)
    }

    /// Create a new syslog writer that sends messages to the well-known UDP port (514).
    pub fn udp_well_known() -> io::Result<SyslogBuilder> {
        fasyslog::sender::udp_well_known()
            .map(SyslogSender::Udp)
            .map(Self::new)
    }

    /// Create a new syslog writer that sends messages to the given UDP address.
    pub fn udp<L: std::net::ToSocketAddrs, R: std::net::ToSocketAddrs>(
        local: L,
        remote: R,
    ) -> io::Result<SyslogBuilder> {
        fasyslog::sender::udp(local, remote)
            .map(SyslogSender::Udp)
            .map(Self::new)
    }

    /// Create a new syslog writer that broadcast messages to the well-known UDP port (514).
    pub fn broadcast_well_known() -> io::Result<SyslogBuilder> {
        fasyslog::sender::broadcast_well_known()
            .map(SyslogSender::Udp)
            .map(Self::new)
    }

    /// Create a new syslog writer that broadcast messages to the given UDP address.
    pub fn broadcast(port: u16) -> io::Result<SyslogBuilder> {
        fasyslog::sender::broadcast(port)
            .map(SyslogSender::Udp)
            .map(Self::new)
    }

    /// Create a TLS sender that sends messages to the well-known port (6514).
    #[cfg(feature = "native-tls")]
    pub fn native_tls_well_known<S: AsRef<str>>(domain: S) -> io::Result<SyslogBuilder> {
        fasyslog::sender::native_tls_well_known(domain)
            .map(SyslogSender::NativeTlsSender)
            .map(Self::new)
    }

    /// Create a TLS sender that sends messages to the given address.
    #[cfg(feature = "native-tls")]
    pub fn native_tls<A: std::net::ToSocketAddrs, S: AsRef<str>>(
        addr: A,
        domain: S,
    ) -> io::Result<SyslogBuilder> {
        fasyslog::sender::native_tls(addr, domain)
            .map(SyslogSender::NativeTlsSender)
            .map(Self::new)
    }

    /// Create a TLS sender that sends messages to the given address with certificate builder.
    #[cfg(feature = "native-tls")]
    pub fn native_tls_with<A: std::net::ToSocketAddrs, S: AsRef<str>>(
        addr: A,
        domain: S,
        builder: native_tls::TlsConnectorBuilder,
    ) -> io::Result<SyslogBuilder> {
        fasyslog::sender::native_tls_with(addr, domain, builder)
            .map(SyslogSender::NativeTlsSender)
            .map(Self::new)
    }

    /// Create a new syslog writer that sends messages to the given Unix stream socket.
    #[cfg(unix)]
    pub fn unix_stream(path: impl AsRef<std::path::Path>) -> io::Result<SyslogBuilder> {
        fasyslog::sender::unix_stream(path)
            .map(SyslogSender::UnixStream)
            .map(Self::new)
    }

    /// Create a new syslog writer that sends messages to the given Unix datagram socket.
    #[cfg(unix)]
    pub fn unix_datagram(path: impl AsRef<std::path::Path>) -> io::Result<SyslogBuilder> {
        fasyslog::sender::unix_datagram(path)
            .map(SyslogSender::UnixDatagram)
            .map(Self::new)
    }

    /// Create a new syslog writer that sends messages to the given Unix socket.
    ///
    /// This method will automatically choose between `unix_stream` and `unix_datagram` based on the
    /// path.
    #[cfg(unix)]
    pub fn unix(path: impl AsRef<std::path::Path>) -> io::Result<SyslogBuilder> {
        fasyslog::sender::unix(path).map(Self::new)
    }
}

/// An appender that writes log records to syslog.
#[derive(Debug)]
pub struct Syslog {
    writer: NonBlocking<SyslogWriter>,
    formatter: SyslogFormatter,
}

impl Syslog {
    /// Creates a new [`Syslog`] appender.
    fn new(writer: NonBlocking<SyslogWriter>, formatter: SyslogFormatter) -> Self {
        Self { writer, formatter }
    }
}

impl Append for Syslog {
    fn append(&self, record: &Record, diagnostics: &[Box<dyn Diagnostic>]) -> anyhow::Result<()> {
        let message = self.formatter.format_message(record, diagnostics)?;
        self.writer.send(message)?;
        Ok(())
    }
}

#[derive(Debug)]
struct SyslogFormatter {
    format: SyslogFormat,
    context: SyslogContext,
    layout: Option<Box<dyn Layout>>,
}

fn log_level_to_otel_severity(level: log::Level) -> fasyslog::Severity {
    match level {
        log::Level::Error => fasyslog::Severity::ERROR,
        log::Level::Warn => fasyslog::Severity::WARNING,
        log::Level::Info => fasyslog::Severity::NOTICE,
        log::Level::Debug => fasyslog::Severity::INFORMATIONAL,
        log::Level::Trace => fasyslog::Severity::DEBUG,
    }
}

impl SyslogFormatter {
    fn format_message(
        &self,
        record: &Record,
        diagnostics: &[Box<dyn Diagnostic>],
    ) -> anyhow::Result<Vec<u8>> {
        let severity = log_level_to_otel_severity(record.level());

        let message = match self.format {
            SyslogFormat::RFC3164 => match self.layout {
                None => format!(
                    "{}",
                    self.context.format_rfc3164(severity, Some(record.args()))
                ),
                Some(ref layout) => {
                    let message = layout.format(record, diagnostics)?;
                    let message = String::from_utf8_lossy(&message);
                    format!("{}", self.context.format_rfc3164(severity, Some(message)))
                }
            },
            SyslogFormat::RFC5424 => {
                const EMPTY_MSGID: Option<&str> = None;
                const EMPTY_STRUCTURED_DATA: Vec<SDElement> = Vec::new();

                match self.layout {
                    None => format!(
                        "{}",
                        self.context.format_rfc5424(
                            severity,
                            EMPTY_MSGID,
                            EMPTY_STRUCTURED_DATA,
                            Some(record.args())
                        )
                    ),
                    Some(ref layout) => {
                        let message = layout.format(record, diagnostics)?;
                        let message = String::from_utf8_lossy(&message);
                        format!(
                            "{}",
                            self.context.format_rfc5424(
                                severity,
                                EMPTY_MSGID,
                                EMPTY_STRUCTURED_DATA,
                                Some(message)
                            )
                        )
                    }
                }
            }
        };

        Ok(message.into_bytes())
    }
}

/// A writer that writes formatted log records to syslog.
#[derive(Debug)]
struct SyslogWriter {
    sender: SyslogSender,
}

impl Writer for SyslogWriter {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.sender.send_formatted(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.sender.flush()
    }
}
