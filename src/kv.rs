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

// This file is derived from https://github.com/SpriteOvO/spdlog-rs/blob/788bda33/spdlog/src/kv.rs

pub extern crate value_bag;

use std::fmt;
use std::slice;

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
    /// Create a `Key` ref.
    pub fn by_ref(&self) -> Key<'_> {
        Key(self.0.by_ref())
    }
}

impl fmt::Display for KeyOwned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// A collection of key-value pairs.
pub struct KeyValues<'a>(KeyValuesState<'a>);

enum KeyValuesState<'a> {
    Borrowed(&'a [(Key<'a>, Value<'a>)]),
    Owned(&'a [(KeyOwned, ValueOwned)]),
}

impl<'a> KeyValues<'a> {
    /// Get the number of key-value pairs.
    pub fn len(&self) -> usize {
        match self.0 {
            KeyValuesState::Borrowed(p) => p.len(),
            KeyValuesState::Owned(p) => p.len(),
        }
    }

    /// Check if there are no key-value pairs.
    pub fn is_empty(&self) -> bool {
        match self.0 {
            KeyValuesState::Borrowed(p) => p.is_empty(),
            KeyValuesState::Owned(p) => p.is_empty(),
        }
    }

    /// Get an iterator over the key-value pairs.
    pub fn iter(&self) -> KeyValuesIter<'a> {
        match &self.0 {
            KeyValuesState::Borrowed(p) => KeyValuesIter(KeyValuesIterState::Borrowed(p.iter())),
            KeyValuesState::Owned(p) => KeyValuesIter(KeyValuesIterState::Owned(p.iter())),
        }
    }

    /// Visit the key-value pairs with the provided visitor.
    pub fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        for (k, v) in self.iter() {
            visitor.visit(k, v)?;
        }
        Ok(())
    }
}

impl fmt::Debug for KeyValues<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl Clone for KeyValues<'_> {
    fn clone(&self) -> Self {
        match &self.0 {
            KeyValuesState::Borrowed(p) => KeyValues(KeyValuesState::Borrowed(p)),
            KeyValuesState::Owned(p) => KeyValues(KeyValuesState::Owned(p)),
        }
    }
}

impl Default for KeyValues<'_> {
    fn default() -> Self {
        KeyValues(KeyValuesState::Borrowed(&[]))
    }
}

impl<'a> IntoIterator for KeyValues<'a> {
    type Item = (Key<'a>, Value<'a>);
    type IntoIter = KeyValuesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> From<&'a [(Key<'a>, Value<'a>)]> for KeyValues<'a> {
    fn from(kvs: &'a [(Key<'a>, Value<'a>)]) -> Self {
        Self(KeyValuesState::Borrowed(kvs))
    }
}

impl<'a> From<&'a [(KeyOwned, ValueOwned)]> for KeyValues<'a> {
    fn from(kvs: &'a [(KeyOwned, ValueOwned)]) -> Self {
        Self(KeyValuesState::Owned(kvs))
    }
}

/// An iterator over key-value pairs.
pub struct KeyValuesIter<'a>(KeyValuesIterState<'a>);

enum KeyValuesIterState<'a> {
    Borrowed(slice::Iter<'a, (Key<'a>, Value<'a>)>),
    Owned(slice::Iter<'a, (KeyOwned, ValueOwned)>),
}

impl<'a> Iterator for KeyValuesIter<'a> {
    type Item = (Key<'a>, Value<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            KeyValuesIterState::Borrowed(iter) => iter.next().map(|(k, v)| (k.clone(), v.clone())),
            KeyValuesIterState::Owned(iter) => iter.next().map(|(k, v)| (k.by_ref(), v.by_ref())),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            KeyValuesIterState::Borrowed(iter) => iter.size_hint(),
            KeyValuesIterState::Owned(iter) => iter.size_hint(),
        }
    }
}
