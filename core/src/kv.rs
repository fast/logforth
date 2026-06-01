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
    /// The absence of a value.
    None,
    /// A boolean value.
    Bool(bool),
    /// A signed 64-bit integer value.
    I64(i64),
    /// An unsigned 64-bit integer value.
    U64(u64),
    /// A 64-bit floating point value.
    F64(f64),
    /// A signed 128-bit integer value.
    I128(i128),
    /// An unsigned 128-bit integer value.
    U128(u128),
    /// A Unicode character value.
    Char(char),
    /// A string value.
    Str(&'a str),
    /// A list value.
    List(ValueList<'a>),
    /// A map value.
    Map(ValueMap<'a>),
    /// A display-formatted value.
    Display(&'a dyn fmt::Display),
    /// A debug-formatted value.
    Debug(&'a dyn fmt::Debug),
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
            Value::List(l) => Value::List(l.clone()),
            Value::Map(m) => Value::Map(m.clone()),
            Value::Display(v) => Value::Display(*v),
            Value::Debug(v) => Value::Debug(*v),
        }
    }
}

impl<'a> Value<'a> {
    /// Create a list value from borrowed values.
    pub fn list(values: &'a [Value<'a>]) -> Self {
        Self::List(ValueList(ValueListState::Borrowed(values)))
    }

    /// Create a map value from borrowed key-values.
    pub fn map(values: &'a [(Key<'a>, Value<'a>)]) -> Self {
        Self::Map(ValueMap(ValueMapState::Borrowed(values)))
    }

    /// Create a value from a `str`.
    pub fn from_str(value: &'a str) -> Self {
        Self::Str(value)
    }

    /// Create a value from a `bool`.
    pub fn from_bool(value: bool) -> Self {
        Self::Bool(value)
    }

    /// Create a value from a type implementing [`fmt::Display`].
    pub fn from_display<T>(value: &'a T) -> Self
    where
        T: fmt::Display,
    {
        Self::Display(value)
    }

    /// Create a value from a type implementing [`fmt::Debug`].
    pub fn from_debug<T>(value: &'a T) -> Self
    where
        T: fmt::Debug,
    {
        Self::Debug(value)
    }

    /// Convert to an owned value.
    pub fn to_owned(&self) -> ValueOwned {
        match self {
            Value::None => ValueOwned::None,
            Value::Bool(v) => ValueOwned::Bool(*v),
            Value::I64(v) => ValueOwned::I64(*v),
            Value::U64(v) => ValueOwned::U64(*v),
            Value::F64(v) => ValueOwned::F64(*v),
            Value::I128(v) => ValueOwned::I128(*v),
            Value::U128(v) => ValueOwned::U128(*v),
            Value::Char(v) => ValueOwned::Char(*v),
            Value::Str(v) => ValueOwned::Str((*v).to_owned()),
            Value::List(v) => ValueOwned::List(Box::new(v.iter().map(|v| v.to_owned()).collect())),
            Value::Map(v) => ValueOwned::Map(Box::new(
                v.iter().map(|(k, v)| (k.to_owned(), v.to_owned())).collect(),
            )),
            Value::Display(v) => ValueOwned::Str(v.to_string()),
            Value::Debug(v) => ValueOwned::Str(format!("{v:?}")),
        }
    }

    /// Try to convert this value into a `bool`.
    pub fn to_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(value) => Some(*value),
            _ => None,
        }
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(value: &'a str) -> Self {
        Value::Str(value)
    }
}

impl<'a> From<&'a String> for Value<'a> {
    fn from(value: &'a String) -> Self {
        Value::Str(value)
    }
}

macro_rules! impl_value_from {
    ($($ty:ty => $variant:ident,)*) => {
        $(
            impl From<$ty> for Value<'_> {
                fn from(value: $ty) -> Self {
                    Value::$variant(value)
                }
            }

            impl From<&$ty> for Value<'_> {
                fn from(value: &$ty) -> Self {
                    Value::$variant(*value)
                }
            }
        )*
    };
}

impl_value_from! {
    bool => Bool,
    i64 => I64,
    u64 => U64,
    f64 => F64,
    i128 => I128,
    u128 => U128,
    char => Char,
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
            Value::Display(v) => fmt::Display::fmt(v, f),

            // implement
            Value::None => f.debug_tuple("None").finish(),
            Value::List(v) => f.debug_list().entries(v.iter()).finish(),
            Value::Map(m) => f.debug_map().entries(m.iter()).finish(),
            Value::Debug(v) => fmt::Debug::fmt(v, f),
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
            Value::Display(v) => fmt::Display::fmt(v, f),
            Value::Debug(v) => fmt::Debug::fmt(v, f),
            v => fmt::Debug::fmt(v, f),
        }
    }
}

/// A list value.
#[derive(Clone)]
pub struct ValueList<'a>(ValueListState<'a>);

#[derive(Clone)]
enum ValueListState<'a> {
    Borrowed(&'a [Value<'a>]),
    Owned(&'a [ValueOwned]),
}

impl<'a> ValueList<'a> {
    /// Get an iterator over the list values.
    pub fn iter(&self) -> ValueListIter<'a> {
        match self.0 {
            ValueListState::Borrowed(values) => {
                ValueListIter(ValueListIterState::Borrowed(values.iter()))
            }
            ValueListState::Owned(values) => ValueListIter(ValueListIterState::Owned(values.iter())),
        }
    }
}

/// An iterator over list values.
pub struct ValueListIter<'a>(ValueListIterState<'a>);

enum ValueListIterState<'a> {
    Borrowed(slice::Iter<'a, Value<'a>>),
    Owned(slice::Iter<'a, ValueOwned>),
}

impl<'a> Iterator for ValueListIter<'a> {
    type Item = Value<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            ValueListIterState::Borrowed(iter) => iter.next().cloned(),
            ValueListIterState::Owned(iter) => iter.next().map(ValueOwned::by_ref),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            ValueListIterState::Borrowed(iter) => iter.size_hint(),
            ValueListIterState::Owned(iter) => iter.size_hint(),
        }
    }
}

/// A map value.
#[derive(Clone)]
pub struct ValueMap<'a>(ValueMapState<'a>);

#[derive(Clone)]
enum ValueMapState<'a> {
    Borrowed(&'a [(Key<'a>, Value<'a>)]),
    Owned(&'a [(KeyOwned, ValueOwned)]),
}

impl<'a> ValueMap<'a> {
    /// Get an iterator over the map key-values.
    pub fn iter(&self) -> ValueMapIter<'a> {
        match self.0 {
            ValueMapState::Borrowed(values) => {
                ValueMapIter(ValueMapIterState::Borrowed(values.iter()))
            }
            ValueMapState::Owned(values) => ValueMapIter(ValueMapIterState::Owned(values.iter())),
        }
    }
}

/// An iterator over map key-values.
pub struct ValueMapIter<'a>(ValueMapIterState<'a>);

enum ValueMapIterState<'a> {
    Borrowed(slice::Iter<'a, (Key<'a>, Value<'a>)>),
    Owned(slice::Iter<'a, (KeyOwned, ValueOwned)>),
}

impl<'a> Iterator for ValueMapIter<'a> {
    type Item = (Key<'a>, Value<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            ValueMapIterState::Borrowed(iter) => iter.next().map(|(k, v)| (k.clone(), v.clone())),
            ValueMapIterState::Owned(iter) => iter.next().map(|(k, v)| (k.by_ref(), v.by_ref())),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            ValueMapIterState::Borrowed(iter) => iter.size_hint(),
            ValueMapIterState::Owned(iter) => iter.size_hint(),
        }
    }
}

/// An owned value in a key-value pair.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ValueOwned {
    /// The absence of a value.
    None,
    /// A boolean value.
    Bool(bool),
    /// A signed 64-bit integer value.
    I64(i64),
    /// An unsigned 64-bit integer value.
    U64(u64),
    /// A 64-bit floating point value.
    F64(f64),
    /// A signed 128-bit integer value.
    I128(i128),
    /// An unsigned 128-bit integer value.
    U128(u128),
    /// A Unicode character value.
    Char(char),
    /// A string value.
    Str(String),
    /// A list value.
    List(Box<Vec<ValueOwned>>),
    /// A map value.
    Map(Box<Vec<(KeyOwned, ValueOwned)>>),
}

impl ValueOwned {
    /// Create a borrowed view of this owned value.
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
            ValueOwned::List(l) => Value::List(ValueList(ValueListState::Owned(l.as_slice()))),
            ValueOwned::Map(m) => Value::Map(ValueMap(ValueMapState::Owned(m.as_slice()))),
        }
    }
}

/// An owned key in a key-value pair.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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
