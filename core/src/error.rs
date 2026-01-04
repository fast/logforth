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

/// The error struct of logforth.
pub struct Error {
    message: String,
    sources: Vec<anyhow::Error>,
    context: Vec<(&'static str, String)>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;

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

        if !self.sources.is_empty() {
            write!(f, ", sources: [")?;
            for (i, source) in self.sources.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{source}")?;
            }
            write!(f, "]")?;
        }

        Ok(())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // If alternate has been specified, we will print like Debug.
        if f.alternate() {
            let mut de = f.debug_struct("Error");
            de.field("message", &self.message);
            de.field("context", &self.context);
            de.field("sources", &self.sources);
            return de.finish();
        }

        write!(f, "{}", self.message)?;
        writeln!(f)?;

        if !self.context.is_empty() {
            writeln!(f)?;
            writeln!(f, "Context:")?;
            for (k, v) in self.context.iter() {
                writeln!(f, "   {k}: {v}")?;
            }
        }
        if !self.sources.is_empty() {
            writeln!(f)?;
            writeln!(f, "Sources:")?;
            for source in self.sources.iter() {
                writeln!(f, "   {source:#}")?;
            }
        }

        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.sources.first().map(|v| v.as_ref())
    }
}

impl Error {
    /// Create a new Error with error kind and message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            sources: vec![],
            context: vec![],
        }
    }

    /// Add one more context in error.
    pub fn with_context(mut self, key: &'static str, value: impl ToString) -> Self {
        self.context.push((key, value.to_string()));
        self
    }

    /// Add one more source in error.
    pub fn with_source(mut self, src: impl Into<anyhow::Error>) -> Self {
        self.sources.push(src.into());
        self
    }

    /// Return an iterator over all sources of this error.
    pub fn sources(&self) -> impl ExactSizeIterator<Item = &(dyn std::error::Error + 'static)> {
        self.sources.iter().map(|v| v.as_ref())
    }

    /// Default constructor for [`Error`] from [`io::Error`].
    pub fn from_io_error(err: io::Error) -> Error {
        Error::new("failed to perform io").with_source(err)
    }

    /// Default constructor for [`Error`] from [`fmt::Error`].
    pub fn from_fmt_error(err: fmt::Error) -> Error {
        Error::new("failed to perform format").with_source(err)
    }
}
