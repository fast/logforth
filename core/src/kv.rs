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

//! Key-value pairs in a log record or a diagnostic context.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::slice;

use crate::Error;
use crate::str::RefStr;

/// A visitor to walk through key-value pairs.
pub trait Visitor {
    /// Visit a key-value pair.
    fn visit(&mut self, key: Key, value: Value) -> Result<(), Error>;
}

/// A key in a key-value pair.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Key<'a>(RefStr<'a>);

impl Key<'static> {
    /// Create a new key from a static `&str`.
    pub const fn new(k: &'static str) -> Key<'static> {
        Key(RefStr::Static(k))
    }
}

impl fmt::Display for Key<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<'a> Key<'a> {
    /// Create a new key from a `&str`.
    ///
    /// The [`Key::new`] method should be preferred where possible.
    pub const fn new_ref(k: &'a str) -> Key<'a> {
        Key(RefStr::Borrowed(k))
    }

    /// Convert to an owned key.
    pub fn to_owned(&self) -> KeyOwned {
        KeyOwned(self.0.into_cow_static())
    }

    /// Convert to a `Cow` str.
    pub fn to_cow(&self) -> Cow<'static, str> {
        self.0.into_cow_static()
    }

    /// Get the key string.
    pub fn as_str(&self) -> &str {
        self.0.get()
    }
}

/// A value in a key-value pair.
#[non_exhaustive]
pub enum Value<'a> {
    None,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    I128(i128),
    U128(u128),
    Char(char),
    Str(&'a str),
    List(&'a [Value<'a>]),
    Map(&'a [(Key<'a>, Value<'a>)]),
}

impl Clone for Value<'_> {
    fn clone(&self) -> Self {
        match self {
            Value::None => Value::None,
            Value::Bool(b) => Value::Bool(*b),
            Value::I64(i) => Value::I64(*i),
            Value::U64(u) => Value::U64(*u),
            Value::F64(f) => Value::F64(*f),
            Value::I128(i) => Value::I128(*i),
            Value::U128(u) => Value::U128(*u),
            Value::Char(c) => Value::Char(*c),
            Value::Str(s) => Value::Str(s),
            Value::List(l) => Value::List(l),
            Value::Map(m) => Value::Map(m),
        }
    }
}

impl fmt::Debug for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // passthrough
            Value::Bool(v) => v.fmt(f),
            Value::I64(v) => v.fmt(f),
            Value::U64(v) => v.fmt(f),
            Value::F64(v) => v.fmt(f),
            Value::I128(v) => v.fmt(f),
            Value::U128(v) => v.fmt(f),
            Value::Char(v) => v.fmt(f),
            Value::Str(v) => v.fmt(f),

            // implement
            Value::None => f.debug_tuple("None").finish(),
            Value::List(v) => f.debug_list().entries(v.iter()).finish(),
            Value::Map(m) => f
                .debug_map()
                .entries(m.iter().map(|(k, v)| (k, v)))
                .finish(),
        }
    }
}

impl fmt::Display for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(v) => v.fmt(f),
            Value::I64(v) => v.fmt(f),
            Value::U64(v) => v.fmt(f),
            Value::F64(v) => v.fmt(f),
            Value::I128(v) => v.fmt(f),
            Value::U128(v) => v.fmt(f),
            Value::Char(v) => v.fmt(f),
            Value::Str(v) => v.fmt(f),
            v => fmt::Debug::fmt(v, f),
        }
    }
}

/// An owned value in a key-value pair.
#[non_exhaustive]
pub enum ValueOwned {
    None,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    I128(i128),
    U128(u128),
    Char(char),
    Str(String),
    List(Box<Vec<ValueOwned>>),
    Map(Box<HashMap<KeyOwned, ValueOwned>>),
}

impl ValueOwned {
    pub fn by_ref(&self) -> Value<'_> {
        match self {
            ValueOwned::None => Value::None,
            ValueOwned::Bool(b) => Value::Bool(*b),
            ValueOwned::I64(i) => Value::I64(*i),
            ValueOwned::U64(u) => Value::U64(*u),
            ValueOwned::F64(f) => Value::F64(*f),
            ValueOwned::I128(i) => Value::I128(*i),
            ValueOwned::U128(u) => Value::U128(*u),
            ValueOwned::Char(c) => Value::Char(*c),
            ValueOwned::Str(s) => Value::Str(s.as_str()),
            ValueOwned::List(l) => {
                let l = l.iter().map(|v| v.by_ref()).collect::<Vec<_>>();
                Value::List(l.as_slice())
            }
            ValueOwned::Map(m) => {
                let m = m
                    .iter()
                    .map(|(k, v)| (k.by_ref(), v.by_ref()))
                    .collect::<Vec<_>>();
                Value::Map(m.as_slice())
            }
        }
    }
}

/// An owned key in a key-value pair.
pub struct KeyOwned(Cow<'static, str>);

impl KeyOwned {
    /// Create a `Key` ref.
    pub fn by_ref(&self) -> Key<'_> {
        Key(match &self.0 {
            Cow::Borrowed(s) => RefStr::Static(s),
            Cow::Owned(s) => RefStr::Borrowed(s),
        })
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

    /// Get the value for a given key.
    ///
    /// If the key appears multiple times in the source then which key is returned is undetermined.
    pub fn get(&self, key: &str) -> Option<Value<'a>> {
        match &self.0 {
            KeyValuesState::Borrowed(p) => p.iter().find_map(|(k, v)| {
                if k.0.get() != key {
                    None
                } else {
                    Some(v.clone())
                }
            }),
            KeyValuesState::Owned(p) => p.iter().find_map(|(k, v)| {
                if k.0.as_ref() != key {
                    None
                } else {
                    Some(v.by_ref())
                }
            }),
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
