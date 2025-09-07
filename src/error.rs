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

use std::fmt;
use std::io;

/// ErrorKind is all kinds of Error of a Logforth component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The component does not know what happened here, and no actions other than
    /// just returning it back.
    Unexpected,

    /// The component does not support this operation.
    Unsupported,

    /// The config for the component is invalid.
    ConfigInvalid,
}

impl ErrorKind {
    /// Convert self into static str.
    pub fn into_static(self) -> &'static str {
        self.into()
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.into_static())
    }
}

impl From<ErrorKind> for &'static str {
    fn from(v: ErrorKind) -> &'static str {
        match v {
            ErrorKind::Unexpected => "Unexpected",
            ErrorKind::Unsupported => "Unsupported",
            ErrorKind::ConfigInvalid => "ConfigInvalid",
        }
    }
}

/// Error is the error struct returned by Logforth's interfaces.
pub struct Error {
    kind: ErrorKind,
    message: String,
    source: Option<anyhow::Error>,
    context: Vec<(&'static str, String)>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;

        if !self.context.is_empty() {
            write!(f, ", context: {{ ")?;
            write!(
                f,
                "{}",
                self.context
                    .iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
            write!(f, " }}")?;
        }

        if !self.message.is_empty() {
            write!(f, " => {}", self.message)?;
        }

        if let Some(source) = &self.source {
            write!(f, ", source: {source}")?;
        }

        Ok(())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // If alternate has been specified, we will print like Debug.
        if f.alternate() {
            let mut de = f.debug_struct("Error");
            de.field("kind", &self.kind);
            de.field("message", &self.message);
            de.field("context", &self.context);
            de.field("source", &self.source);
            return de.finish();
        }

        write!(f, "{}", self.kind)?;
        if !self.message.is_empty() {
            write!(f, " => {}", self.message)?;
        }
        writeln!(f)?;

        if !self.context.is_empty() {
            writeln!(f)?;
            writeln!(f, "Context:")?;
            for (k, v) in self.context.iter() {
                writeln!(f, "   {k}: {v}")?;
            }
        }
        if let Some(source) = &self.source {
            writeln!(f)?;
            writeln!(f, "Source:")?;
            writeln!(f, "   {source:#}")?;
        }

        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|v| v.as_ref())
    }
}

impl Error {
    /// Create a new Error with error kind and message.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
            context: vec![],
        }
    }

    /// Add more context in error.
    pub fn with_context(mut self, key: &'static str, value: impl ToString) -> Self {
        self.context.push((key, value.to_string()));
        self
    }

    /// Set source for error.
    ///
    /// # Notes
    ///
    /// If the source has been set, we will raise a panic here.
    pub fn set_source(mut self, src: impl Into<anyhow::Error>) -> Self {
        debug_assert!(self.source.is_none(), "the source error has been set");

        self.source = Some(src.into());
        self
    }

    /// Return the kind of this error.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Default constructor for [`Error`] from [`io::Error`].
    pub fn from_io_error(err: io::Error) -> Error {
        Error::new(ErrorKind::Unexpected, "failed to perform io").set_source(err)
    }

    /// Default constructor for [`Error`] from [`fmt::Error`].
    pub fn from_fmt_error(err: fmt::Error) -> Error {
        Error::new(ErrorKind::Unexpected, "failed to perform format").set_source(err)
    }

    /// Default constructor for [`Error`] from [`log::kv::Error`].
    pub fn from_kv_error(err: log::kv::Error) -> Error {
        Error::new(ErrorKind::Unexpected, "failed to visit log kvs").set_source(err)
    }
}
