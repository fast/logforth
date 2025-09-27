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

//! The module for key-value pairs in a log record or a diagnostic context.

pub extern crate value_bag;

use std::fmt;
use std::fmt::Debug;

use value_bag::OwnedValueBag;
use value_bag::ValueBag;

use crate::Error;
use crate::Str;

/// A visitor to walk through key-value pairs.
pub trait Visitor {
    /// Visits a key-value pair.
    fn visit(&mut self, key: Key, value: Value) -> Result<(), Error>;
}

/// Represent a value in a key-value pair.
pub type Value<'a> = ValueBag<'a>;

/// Represent a key in a key-value pair.
#[derive(Debug, Clone)]
pub struct Key<'a>(Str<'a>);

impl<'a> Key<'a> {
    /// Convert to an owned `String`.
    pub fn into_string(self) -> String {
        self.0.into_string()
    }

    /// Convert to an owned key.
    pub fn to_owned(&self) -> KeyOwned {
        KeyOwned(self.0.to_owned())
    }

    /// Get the key string.
    pub fn as_str(&self) -> &str {
        self.0.get()
    }

    /// Coerce to a key with a shorter lifetime.
    pub fn coerce(&self) -> Key<'_> {
        Key(self.0.by_ref())
    }
}

impl fmt::Display for Key<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<'a> From<Str<'a>> for Key<'a> {
    fn from(s: Str<'a>) -> Self {
        Key(s)
    }
}

impl<'a> From<&'a str> for Key<'a> {
    fn from(s: &'a str) -> Self {
        Key(Str::from(s))
    }
}

/// Represent an owned value in a key-value pair.
pub type ValueOwned = OwnedValueBag;

/// Represent an owned key in a key-value pair.
#[derive(Debug, Clone)]
pub struct KeyOwned(Str<'static>);

impl KeyOwned {
    /// Get the owned key string.
    pub fn as_ref(&self) -> Key<'_> {
        Key(self.0.by_ref())
    }
}

impl fmt::Display for KeyOwned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
