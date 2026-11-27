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
//!
//! Keys prefixed with `logforth.` are reserved by Logforth; avoid using `logforth.*` for
//! application-defined fields unless intentionally invoking a documented behavior.

use std::borrow::Borrow;
use std::borrow::Cow;
use std::fmt;
use std::slice;

use crate::Error;
use crate::str::RefStr;

#[cfg(feature = "serde")]
mod value_serde;

/// A visitor to walk through key-value pairs.
pub trait Visitor {
    /// Visit a key-value pair.
    fn visit(&mut self, key: KeyView, value: ValueView) -> Result<(), Error>;
}

impl<F> Visitor for F
where
    F: FnMut(KeyView, ValueView) -> Result<(), Error>,
{
    fn visit(&mut self, key: KeyView, value: ValueView) -> Result<(), Error> {
        self(key, value)
    }
}

/// A key in a key-value pair.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Key<'a>(RefStr<'a>);

impl fmt::Display for Key<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Key<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl Borrow<str> for Key<'_> {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Key<'static> {
    /// Create a new key from a static `&str`.
    pub const fn new(k: &'static str) -> Key<'static> {
        Key(RefStr::Static(k))
    }
}

impl<'a> Key<'a> {
    /// Create a new key from a `&str`.
    ///
    /// The [`Key::new`] method should be preferred where possible.
    pub const fn borrowed(k: &'a str) -> Key<'a> {
        Key(RefStr::Borrowed(k))
    }

    /// Create a borrowed view of this key.
    pub const fn view(&self) -> KeyView<'a> {
        KeyView(self.0)
    }
}

/// An owned key in a key-value pair.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct KeyOwned(Cow<'static, str>);

macro_rules! impl_key_owned_from {
    ($ty:ty) => {
        impl From<$ty> for KeyOwned {
            fn from(v: $ty) -> Self {
                KeyOwned(Cow::from(v))
            }
        }
    };
}

impl_key_owned_from!(&'static str);
impl_key_owned_from!(String);
impl_key_owned_from!(Cow<'static, str>);

impl Borrow<str> for KeyOwned {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for KeyOwned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for KeyOwned {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl KeyOwned {
    /// Create an owned key.
    pub fn new(k: impl Into<Cow<'static, str>>) -> KeyOwned {
        KeyOwned(k.into())
    }

    /// Create a borrowed view of this owned key.
    pub fn view(&self) -> KeyView<'_> {
        KeyView(match &self.0 {
            Cow::Borrowed(s) => RefStr::Static(s),
            Cow::Owned(s) => RefStr::Borrowed(s),
        })
    }
}

/// A borrowed view of a key in a key-value pair.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct KeyView<'a>(RefStr<'a>);

impl Borrow<str> for KeyView<'_> {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for KeyView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for KeyView<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'a> KeyView<'a> {
    /// Convert to an owned key.
    pub fn to_owned(&self) -> KeyOwned {
        KeyOwned(self.0.into_cow_static())
    }

    /// Convert to a `Cow` str.
    pub fn to_cow(&self) -> Cow<'static, str> {
        self.0.into_cow_static()
    }

    /// Get the key string.
    pub const fn as_str(&self) -> &'a str {
        self.0.get()
    }

    /// Get the key string if it is `'static`.
    pub const fn to_static_str(&self) -> Option<&'static str> {
        self.0.get_static()
    }
}

/// A value captured through its [`fmt::Debug`] representation.
#[derive(Clone, Copy)]
pub struct DebugValue<'a>(&'a dyn fmt::Debug);

impl<'a> DebugValue<'a> {
    /// Create a new [`DebugValue`] from a [`fmt::Debug`] reference.
    pub const fn new(d: &'a dyn fmt::Debug) -> Self {
        Self(d)
    }
}

impl fmt::Debug for DebugValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for DebugValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

/// A value captured through its [`fmt::Display`] representation.
#[derive(Clone, Copy)]
pub struct DisplayValue<'a>(&'a dyn fmt::Display);

impl<'a> DisplayValue<'a> {
    /// Create a new [`DisplayValue`] from a [`fmt::Display`] reference.
    pub const fn new(d: &'a dyn fmt::Display) -> Self {
        Self(d)
    }
}

impl fmt::Debug for DisplayValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Display for DisplayValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// A borrowed view over a list value.
#[derive(Debug, Clone, Copy)]
pub struct ListValue<'a>(ListValueState<'a>);

#[derive(Debug, Clone, Copy)]
enum ListValueState<'a> {
    Borrowed(&'a [Value<'a>]),
    Owned(&'a [ValueOwned]),
}

#[cfg(feature = "serde")]
impl serde::Serialize for ListValue<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
    }
}

impl<'a> ListValue<'a> {
    /// Get the number of elements.
    pub const fn len(&self) -> usize {
        match self.0 {
            ListValueState::Borrowed(p) => p.len(),
            ListValueState::Owned(p) => p.len(),
        }
    }

    /// Check if this is an empty list.
    pub const fn is_empty(&self) -> bool {
        match self.0 {
            ListValueState::Borrowed(p) => p.is_empty(),
            ListValueState::Owned(p) => p.is_empty(),
        }
    }

    /// Get an iterator over the list values.
    pub fn iter(&self) -> ListValueIter<'a> {
        match self.0 {
            ListValueState::Borrowed(v) => ListValueIter(ListValueIterState::Borrowed(v.iter())),
            ListValueState::Owned(v) => ListValueIter(ListValueIterState::Owned(v.iter())),
        }
    }
}

/// An iterator over list values.
#[derive(Debug, Clone)]
pub struct ListValueIter<'a>(ListValueIterState<'a>);

#[derive(Debug, Clone)]
enum ListValueIterState<'a> {
    Borrowed(slice::Iter<'a, Value<'a>>),
    Owned(slice::Iter<'a, ValueOwned>),
}

impl<'a> Iterator for ListValueIter<'a> {
    type Item = ValueView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            ListValueIterState::Borrowed(v) => v.next().map(Value::view),
            ListValueIterState::Owned(v) => v.next().map(ValueOwned::view),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            ListValueIterState::Borrowed(v) => v.size_hint(),
            ListValueIterState::Owned(v) => v.size_hint(),
        }
    }
}

/// A borrowed view over a map value.
#[derive(Debug, Clone, Copy)]
pub struct MapValue<'a>(MapValueState<'a>);

#[derive(Debug, Clone, Copy)]
enum MapValueState<'a> {
    Borrowed(&'a [(Key<'a>, Value<'a>)]),
    Owned(&'a [(KeyOwned, ValueOwned)]),
}

#[cfg(feature = "serde")]
impl serde::Serialize for MapValue<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(self.iter())
    }
}

impl<'a> MapValue<'a> {
    /// Get the number of key-values.
    pub const fn len(&self) -> usize {
        match self.0 {
            MapValueState::Borrowed(p) => p.len(),
            MapValueState::Owned(p) => p.len(),
        }
    }

    /// Check if there are no key-value pairs.
    pub const fn is_empty(&self) -> bool {
        match self.0 {
            MapValueState::Borrowed(p) => p.is_empty(),
            MapValueState::Owned(p) => p.is_empty(),
        }
    }

    /// Get an iterator over the map key-value pairs in insertion order.
    pub fn iter(&self) -> MapValueIter<'a> {
        match self.0 {
            MapValueState::Borrowed(v) => MapValueIter(MapValueIterState::Borrowed(v.iter())),
            MapValueState::Owned(v) => MapValueIter(MapValueIterState::Owned(v.iter())),
        }
    }

    /// Get the value for `key`, returning the last matching entry if duplicate keys exist.
    pub fn get(&self, key: &str) -> Option<ValueView<'a>> {
        match self.0 {
            MapValueState::Borrowed(p) => p
                .iter()
                .rev()
                .find_map(|(k, v)| (k.view().as_str() == key).then(|| v.view())),
            MapValueState::Owned(p) => p
                .iter()
                .rev()
                .find_map(|(k, v)| (k.view().as_str() == key).then(|| v.view())),
        }
    }
}

/// An iterator over map key-value pairs.
#[derive(Debug, Clone)]
pub struct MapValueIter<'a>(MapValueIterState<'a>);

#[derive(Debug, Clone)]
enum MapValueIterState<'a> {
    Borrowed(slice::Iter<'a, (Key<'a>, Value<'a>)>),
    Owned(slice::Iter<'a, (KeyOwned, ValueOwned)>),
}

impl<'a> Iterator for MapValueIter<'a> {
    type Item = (KeyView<'a>, ValueView<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            MapValueIterState::Borrowed(v) => v.next().map(|(k, v)| (k.view(), v.view())),
            MapValueIterState::Owned(v) => v.next().map(|(k, v)| (k.view(), v.view())),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            MapValueIterState::Borrowed(v) => v.size_hint(),
            MapValueIterState::Owned(v) => v.size_hint(),
        }
    }
}

/// A borrowed value in a key-value pair.
#[derive(Debug, Clone, Copy)]
pub struct Value<'a>(ValueState<'a>);

#[derive(Clone, Copy)]
enum ValueState<'a> {
    None,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    I128(i128),
    U128(u128),
    Char(char),
    Str(RefStr<'a>),
    Bytes(&'a [u8]),
    List(&'a [Value<'a>]),
    Map(&'a [(Key<'a>, Value<'a>)]),
    Owned(&'a ValueOwned),
    Debug(&'a (dyn fmt::Debug + Sync)),
    Display(&'a (dyn fmt::Display + Sync)),
}

impl fmt::Debug for ValueState<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueState::None => write!(f, "<none>"),
            ValueState::Bool(v) => v.fmt(f),
            ValueState::I64(v) => v.fmt(f),
            ValueState::U64(v) => v.fmt(f),
            ValueState::F64(v) => v.fmt(f),
            ValueState::I128(v) => v.fmt(f),
            ValueState::U128(v) => v.fmt(f),
            ValueState::Char(v) => v.fmt(f),
            ValueState::Str(v) => v.fmt(f),
            ValueState::Bytes(v) => v.fmt(f),
            ValueState::List(v) => v.fmt(f),
            ValueState::Map(v) => v.fmt(f),
            ValueState::Owned(v) => v.fmt(f),
            ValueState::Debug(v) => fmt::Debug::fmt(v, f),
            ValueState::Display(v) => fmt::Display::fmt(v, f),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Value<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.view().serialize(serializer)
    }
}

impl<'a> Value<'a> {
    /// Create a borrowed view of this value.
    pub fn view(&self) -> ValueView<'a> {
        match self.0 {
            ValueState::None => ValueView::None,
            ValueState::Bool(b) => ValueView::Bool(b),
            ValueState::I64(i) => ValueView::I64(i),
            ValueState::U64(u) => ValueView::U64(u),
            ValueState::F64(f) => ValueView::F64(f),
            ValueState::I128(i) => ValueView::I128(i),
            ValueState::U128(u) => ValueView::U128(u),
            ValueState::Char(c) => ValueView::Char(c),
            ValueState::Str(RefStr::Static(s)) => ValueView::StaticStr(s),
            ValueState::Str(RefStr::Borrowed(s)) => ValueView::BorrowedStr(s),
            ValueState::Bytes(b) => ValueView::Bytes(b),
            ValueState::List(l) => ValueView::List(ListValue(ListValueState::Borrowed(l))),
            ValueState::Map(m) => ValueView::Map(MapValue(MapValueState::Borrowed(m))),
            ValueState::Owned(o) => o.view(),
            ValueState::Debug(d) => ValueView::Debug(DebugValue(d)),
            ValueState::Display(d) => ValueView::Display(DisplayValue(d)),
        }
    }

    /// Create a value representing the absence of data.
    pub const fn none() -> Value<'a> {
        Value(ValueState::None)
    }

    /// Create a value from a borrowed string.
    pub const fn str(s: &'a str) -> Self {
        Value(ValueState::Str(RefStr::Borrowed(s)))
    }

    /// Create a value from a static string.
    pub const fn static_str(s: &'static str) -> Self {
        Value(ValueState::Str(RefStr::Static(s)))
    }

    /// Create a value from a byte slice.
    pub const fn bytes(b: &'a [u8]) -> Self {
        Value(ValueState::Bytes(b))
    }

    /// Create a value from a boolean.
    pub const fn bool(b: bool) -> Self {
        Value(ValueState::Bool(b))
    }

    /// Create a value from a signed 64-bit integer.
    pub const fn i64(i: i64) -> Self {
        Value(ValueState::I64(i))
    }

    /// Create a value from an unsigned 64-bit integer.
    pub const fn u64(u: u64) -> Self {
        Value(ValueState::U64(u))
    }

    /// Create a value from a 64-bit floating point number.
    pub const fn f64(f: f64) -> Self {
        Value(ValueState::F64(f))
    }

    /// Create a value from a signed 128-bit integer.
    pub const fn i128(i: i128) -> Self {
        Value(ValueState::I128(i))
    }

    /// Create a value from an unsigned 128-bit integer.
    pub const fn u128(u: u128) -> Self {
        Value(ValueState::U128(u))
    }

    /// Create a value from a Unicode scalar value.
    pub const fn char(c: char) -> Self {
        Value(ValueState::Char(c))
    }

    /// Create a value from a borrowed list of values.
    pub const fn list(l: &'a [Value<'a>]) -> Self {
        Value(ValueState::List(l))
    }

    /// Create a value from a borrowed map of key-value pairs.
    pub const fn map(m: &'a [(Key<'a>, Value<'a>)]) -> Self {
        Value(ValueState::Map(m))
    }

    /// Create a value borrowing an owned [`ValueOwned`].
    pub const fn owned(v: &'a ValueOwned) -> Self {
        Value(ValueState::Owned(v))
    }

    /// Create a value that is formatted lazily with [`fmt::Debug`].
    pub const fn debug(d: &'a (dyn fmt::Debug + Sync)) -> Self {
        Value(ValueState::Debug(d))
    }

    /// Create a value that is formatted lazily with [`fmt::Display`].
    pub const fn display(d: &'a (dyn fmt::Display + Sync)) -> Self {
        Value(ValueState::Display(d))
    }
}

macro_rules! impl_value_scalar_from {
    ($ty:ty, |$v:ident| $expr:expr) => {
        impl From<$ty> for Value<'_> {
            fn from($v: $ty) -> Self {
                $expr
            }
        }

        impl<'a> From<&'a $ty> for Value<'a> {
            fn from($v: &'a $ty) -> Self {
                let $v = *$v;
                $expr
            }
        }
    };
}

impl_value_scalar_from!(bool, |v| Value::bool(v));
impl_value_scalar_from!(char, |v| Value::char(v));
impl_value_scalar_from!(i8, |v| Value::i64(i64::from(v)));
impl_value_scalar_from!(i16, |v| Value::i64(i64::from(v)));
impl_value_scalar_from!(i32, |v| Value::i64(i64::from(v)));
impl_value_scalar_from!(i64, |v| Value::i64(v));
impl_value_scalar_from!(i128, |v| Value::i128(v));
impl_value_scalar_from!(isize, |v| Value::i64(v as i64));
impl_value_scalar_from!(u8, |v| Value::u64(u64::from(v)));
impl_value_scalar_from!(u16, |v| Value::u64(u64::from(v)));
impl_value_scalar_from!(u32, |v| Value::u64(u64::from(v)));
impl_value_scalar_from!(u64, |v| Value::u64(v));
impl_value_scalar_from!(u128, |v| Value::u128(v));
impl_value_scalar_from!(usize, |v| Value::u64(v as u64));
impl_value_scalar_from!(f32, |v| Value::f64(f64::from(v)));
impl_value_scalar_from!(f64, |v| Value::f64(v));

impl<'a> From<&'a Value<'a>> for Value<'a> {
    fn from(v: &'a Value<'a>) -> Self {
        *v
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(v: &'a str) -> Self {
        Value::str(v)
    }
}

impl<'a> From<&'a String> for Value<'a> {
    fn from(v: &'a String) -> Self {
        Value::str(v.as_str())
    }
}

impl<'a> From<&'a Cow<'_, str>> for Value<'a> {
    fn from(v: &'a Cow<'_, str>) -> Self {
        Value::str(v.as_ref())
    }
}

impl<'a> From<&'a [u8]> for Value<'a> {
    fn from(v: &'a [u8]) -> Self {
        Value::bytes(v)
    }
}

impl<'a, const N: usize> From<&'a [u8; N]> for Value<'a> {
    fn from(v: &'a [u8; N]) -> Self {
        Value::bytes(v.as_slice())
    }
}

impl<'a> From<&'a Vec<u8>> for Value<'a> {
    fn from(v: &'a Vec<u8>) -> Self {
        Value::bytes(v.as_slice())
    }
}

impl<'a> From<&'a ValueOwned> for Value<'a> {
    fn from(v: &'a ValueOwned) -> Self {
        Value::owned(v)
    }
}

impl<'a> From<&'a [Value<'a>]> for Value<'a> {
    fn from(v: &'a [Value<'a>]) -> Self {
        Value::list(v)
    }
}

impl<'a> From<&'a [(Key<'a>, Value<'a>)]> for Value<'a> {
    fn from(v: &'a [(Key<'a>, Value<'a>)]) -> Self {
        Value::map(v)
    }
}

impl<'a, T: Into<Value<'a>>> From<Option<T>> for Value<'a> {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Value::none(),
        }
    }
}

/// An owned value in a key-value pair.
#[derive(Debug, Clone)]
pub struct ValueOwned(ValueOwnedState);

#[derive(Debug, Clone)]
enum ValueOwnedState {
    None,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    I128(i128),
    U128(u128),
    Char(char),
    Str(Cow<'static, str>),
    Bytes(Box<[u8]>),
    List(Box<[ValueOwned]>),
    Map(Box<[(KeyOwned, ValueOwned)]>),
}

macro_rules! impl_value_owned_from {
    ($ty:ty, |$v:ident| $expr:expr) => {
        impl From<$ty> for ValueOwned {
            fn from($v: $ty) -> Self {
                $expr
            }
        }
    };
}

impl_value_owned_from!(bool, |v| Self::bool(v));
impl_value_owned_from!(char, |v| Self::char(v));
impl_value_owned_from!(i8, |v| Self::i64(i64::from(v)));
impl_value_owned_from!(i16, |v| Self::i64(i64::from(v)));
impl_value_owned_from!(i32, |v| Self::i64(i64::from(v)));
impl_value_owned_from!(i64, |v| Self::i64(v));
impl_value_owned_from!(i128, |v| Self::i128(v));
impl_value_owned_from!(isize, |v| Self::i64(v as i64));
impl_value_owned_from!(u8, |v| Self::u64(u64::from(v)));
impl_value_owned_from!(u16, |v| Self::u64(u64::from(v)));
impl_value_owned_from!(u32, |v| Self::u64(u64::from(v)));
impl_value_owned_from!(u64, |v| Self::u64(v));
impl_value_owned_from!(u128, |v| Self::u128(v));
impl_value_owned_from!(usize, |v| Self::u64(v as u64));
impl_value_owned_from!(f32, |v| Self::f64(f64::from(v)));
impl_value_owned_from!(f64, |v| Self::f64(v));
impl_value_owned_from!(String, |v| Self::str(v));
impl_value_owned_from!(&'static str, |v| Self::str(v));
impl_value_owned_from!(Cow<'static, str>, |v| Self::str(v));
impl_value_owned_from!(Vec<u8>, |v| Self::bytes(v));
impl_value_owned_from!(Box<[u8]>, |v| Self::bytes(v));

impl<T: Into<ValueOwned>> From<Option<T>> for ValueOwned {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Self::none(),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for ValueOwned {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.view().serialize(serializer)
    }
}

impl ValueOwned {
    /// Create a borrowed view of this owned value.
    pub fn view(&self) -> ValueView<'_> {
        match &self.0 {
            ValueOwnedState::None => ValueView::None,
            ValueOwnedState::Bool(b) => ValueView::Bool(*b),
            ValueOwnedState::I64(i) => ValueView::I64(*i),
            ValueOwnedState::U64(u) => ValueView::U64(*u),
            ValueOwnedState::F64(f) => ValueView::F64(*f),
            ValueOwnedState::I128(i) => ValueView::I128(*i),
            ValueOwnedState::U128(u) => ValueView::U128(*u),
            ValueOwnedState::Char(c) => ValueView::Char(*c),
            ValueOwnedState::Str(Cow::Borrowed(s)) => ValueView::StaticStr(s),
            ValueOwnedState::Str(Cow::Owned(s)) => ValueView::BorrowedStr(s.as_str()),
            ValueOwnedState::Bytes(b) => ValueView::Bytes(b),
            ValueOwnedState::List(l) => ValueView::List(ListValue(ListValueState::Owned(l))),
            ValueOwnedState::Map(m) => ValueView::Map(MapValue(MapValueState::Owned(m))),
        }
    }

    /// Create an owned value representing the absence of data.
    pub const fn none() -> ValueOwned {
        ValueOwned(ValueOwnedState::None)
    }

    /// Create an owned value from a boolean.
    pub const fn bool(b: bool) -> ValueOwned {
        ValueOwned(ValueOwnedState::Bool(b))
    }

    /// Create an owned value from a signed 64-bit integer.
    pub const fn i64(i: i64) -> ValueOwned {
        ValueOwned(ValueOwnedState::I64(i))
    }

    /// Create an owned value from an unsigned 64-bit integer.
    pub const fn u64(u: u64) -> ValueOwned {
        ValueOwned(ValueOwnedState::U64(u))
    }

    /// Create an owned value from a 64-bit floating point number.
    pub const fn f64(f: f64) -> ValueOwned {
        ValueOwned(ValueOwnedState::F64(f))
    }

    /// Create an owned value from a signed 128-bit integer.
    pub const fn i128(i: i128) -> ValueOwned {
        ValueOwned(ValueOwnedState::I128(i))
    }

    /// Create an owned value from an unsigned 128-bit integer.
    pub const fn u128(u: u128) -> ValueOwned {
        ValueOwned(ValueOwnedState::U128(u))
    }

    /// Create an owned value from a Unicode scalar value.
    pub const fn char(c: char) -> ValueOwned {
        ValueOwned(ValueOwnedState::Char(c))
    }

    /// Create an owned value from a string.
    pub fn str(s: impl Into<Cow<'static, str>>) -> ValueOwned {
        ValueOwned(ValueOwnedState::Str(s.into()))
    }

    /// Create an owned value from bytes.
    pub fn bytes(b: impl Into<Box<[u8]>>) -> ValueOwned {
        ValueOwned(ValueOwnedState::Bytes(b.into()))
    }

    /// Create an owned value from a list of owned values.
    pub fn list(l: impl IntoIterator<Item = ValueOwned>) -> ValueOwned {
        ValueOwned(ValueOwnedState::List(l.into_iter().collect()))
    }

    /// Create an owned value from an insertion-ordered sequence of owned key-value pairs.
    pub fn map(m: impl IntoIterator<Item = (KeyOwned, ValueOwned)>) -> ValueOwned {
        ValueOwned(ValueOwnedState::Map(m.into_iter().collect()))
    }
}

/// A borrowed view of a value.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ValueView<'a> {
    /// The absence of a value.
    None,
    /// A borrowed string value.
    BorrowedStr(&'a str),
    /// A static string value.
    StaticStr(&'static str),
    /// A byte slice value.
    Bytes(&'a [u8]),
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
    /// A Unicode scalar value.
    Char(char),
    /// A list value.
    List(ListValue<'a>),
    /// A map value.
    Map(MapValue<'a>),
    /// A lazily debug-formatted value.
    Debug(DebugValue<'a>),
    /// A lazily display-formatted value.
    Display(DisplayValue<'a>),
}

impl fmt::Display for ValueView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueView::None => write!(f, "<none>"),
            ValueView::BorrowedStr(v) => v.fmt(f),
            ValueView::StaticStr(v) => v.fmt(f),
            ValueView::Bytes(v) => {
                // this follows what `bytes` does:
                // https://github.com/tokio-rs/bytes/blob/2256e6dc/src/fmt/debug.rs
                write!(f, "b\"")?;
                for &b in v.iter() {
                    // https://doc.rust-lang.org/reference/tokens.html#byte-escapes
                    if b == b'\n' {
                        write!(f, "\\n")?;
                    } else if b == b'\r' {
                        write!(f, "\\r")?;
                    } else if b == b'\t' {
                        write!(f, "\\t")?;
                    } else if b == b'\\' || b == b'"' {
                        write!(f, "\\{}", b as char)?;
                    } else if b == b'\0' {
                        write!(f, "\\0")?;
                    // ASCII printable
                    } else if (0x20..0x7f).contains(&b) {
                        write!(f, "{}", b as char)?;
                    } else {
                        write!(f, "\\x{:02x}", b)?;
                    }
                }
                write!(f, "\"")?;
                Ok(())
            }
            ValueView::Bool(v) => v.fmt(f),
            ValueView::I64(v) => v.fmt(f),
            ValueView::U64(v) => v.fmt(f),
            ValueView::F64(v) => v.fmt(f),
            ValueView::I128(v) => v.fmt(f),
            ValueView::U128(v) => v.fmt(f),
            ValueView::Char(v) => v.fmt(f),
            ValueView::List(v) => {
                let mut dbg = f.debug_list();
                for item in v.iter() {
                    dbg.entry(&item);
                }
                dbg.finish()
            }
            ValueView::Map(v) => {
                let mut dbg = f.debug_map();
                for (k, v) in v.iter() {
                    dbg.entry(&k, &v);
                }
                dbg.finish()
            }
            ValueView::Debug(v) => fmt::Debug::fmt(v, f),
            ValueView::Display(v) => fmt::Display::fmt(v, f),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for ValueView<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match &self {
            ValueView::None => serializer.serialize_none(),
            ValueView::BorrowedStr(v) => serializer.serialize_str(v),
            ValueView::StaticStr(v) => serializer.serialize_str(v),
            ValueView::Bytes(v) => serializer.serialize_bytes(v),
            ValueView::Bool(v) => serializer.serialize_bool(*v),
            ValueView::I64(v) => serializer.serialize_i64(*v),
            ValueView::U64(v) => serializer.serialize_u64(*v),
            ValueView::F64(v) => serializer.serialize_f64(*v),
            ValueView::I128(v) => serializer.serialize_i128(*v),
            ValueView::U128(v) => serializer.serialize_u128(*v),
            ValueView::Char(v) => serializer.serialize_char(*v),
            ValueView::List(v) => v.serialize(serializer),
            ValueView::Map(v) => v.serialize(serializer),
            ValueView::Debug(v) => serializer.collect_str(v),
            ValueView::Display(v) => serializer.collect_str(v),
        }
    }
}

impl ValueView<'_> {
    /// Convert this view into an owned value.
    pub fn to_owned(&self) -> ValueOwned {
        match self {
            Self::None => ValueOwned::none(),
            Self::BorrowedStr(s) => ValueOwned::str((*s).to_owned()),
            Self::StaticStr(s) => ValueOwned::str(*s),
            Self::Bytes(b) => ValueOwned::bytes(*b),
            Self::Bool(b) => ValueOwned::bool(*b),
            Self::I64(i) => ValueOwned::i64(*i),
            Self::U64(u) => ValueOwned::u64(*u),
            Self::F64(f) => ValueOwned::f64(*f),
            Self::I128(i) => ValueOwned::i128(*i),
            Self::U128(u) => ValueOwned::u128(*u),
            Self::Char(c) => ValueOwned::char(*c),
            Self::List(l) => ValueOwned::list(l.iter().map(|v| v.to_owned())),
            Self::Map(m) => ValueOwned::map(m.iter().map(|(k, v)| (k.to_owned(), v.to_owned()))),
            Self::Debug(d) => ValueOwned::str(format!("{d:?}")),
            Self::Display(d) => ValueOwned::str(d.to_string()),
        }
    }
}

impl<'a> ValueView<'a> {
    /// Try to convert this view into a boolean.
    pub const fn to_bool(&self) -> Option<bool> {
        if let ValueView::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    /// Try to convert this view into a signed 64-bit integer.
    pub const fn to_i64(&self) -> Option<i64> {
        if let ValueView::I64(i) = self {
            Some(*i)
        } else {
            None
        }
    }

    /// Try to convert this view into an unsigned 64-bit integer.
    pub const fn to_u64(&self) -> Option<u64> {
        if let ValueView::U64(u) = self {
            Some(*u)
        } else {
            None
        }
    }

    /// Try to convert this view into a 64-bit floating point number.
    pub const fn to_f64(&self) -> Option<f64> {
        if let ValueView::F64(f) = self {
            Some(*f)
        } else {
            None
        }
    }

    /// Try to convert this view into a signed 128-bit integer.
    pub const fn to_i128(&self) -> Option<i128> {
        if let ValueView::I128(i) = self {
            Some(*i)
        } else {
            None
        }
    }

    /// Try to convert this view into an unsigned 128-bit integer.
    pub const fn to_u128(&self) -> Option<u128> {
        if let ValueView::U128(u) = self {
            Some(*u)
        } else {
            None
        }
    }

    /// Try to convert this view into a Unicode scalar value.
    pub const fn to_char(&self) -> Option<char> {
        if let ValueView::Char(c) = self {
            Some(*c)
        } else {
            None
        }
    }

    /// Try to convert this view into a string slice.
    pub const fn to_str(&self) -> Option<&'a str> {
        if let ValueView::BorrowedStr(s) = self {
            Some(*s)
        } else if let ValueView::StaticStr(s) = self {
            Some(*s)
        } else {
            None
        }
    }

    /// Try to convert this view into a static string slice.
    pub const fn to_static_str(&self) -> Option<&'static str> {
        if let ValueView::StaticStr(s) = self {
            Some(*s)
        } else {
            None
        }
    }

    /// Try to convert this view into a byte slice.
    pub const fn to_bytes(&self) -> Option<&'a [u8]> {
        if let ValueView::Bytes(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    /// Try to convert this view into a display-formatted value.
    pub const fn to_display(&self) -> Option<DisplayValue<'a>> {
        if let ValueView::Display(d) = self {
            Some(*d)
        } else {
            None
        }
    }

    /// Try to convert this view into a list value.
    pub const fn to_list(&self) -> Option<ListValue<'a>> {
        if let ValueView::List(l) = self {
            Some(*l)
        } else {
            None
        }
    }

    /// Try to convert this view into a map value.
    pub const fn to_map(&self) -> Option<MapValue<'a>> {
        if let ValueView::Map(m) = self {
            Some(*m)
        } else {
            None
        }
    }

    /// Try to convert this view into a debug-formatted value.
    pub const fn to_debug(&self) -> Option<DebugValue<'a>> {
        if let ValueView::Debug(d) = self {
            Some(*d)
        } else {
            None
        }
    }
}

/// A collection of key-value pairs.
#[derive(Debug, Clone, Copy)]
pub struct KeyValues<'a>(KeyValuesState<'a>);

#[derive(Debug, Clone, Copy)]
enum KeyValuesState<'a> {
    Borrowed(&'a [(Key<'a>, Value<'a>)]),
    Owned(&'a [(KeyOwned, ValueOwned)]),
}

impl KeyValues<'_> {
    /// Create an empty key-value collection.
    pub const fn empty() -> Self {
        KeyValues(KeyValuesState::Borrowed(&[]))
    }
}

impl<'a> KeyValues<'a> {
    /// Get the number of key-value pairs.
    pub const fn len(&self) -> usize {
        match self.0 {
            KeyValuesState::Borrowed(p) => p.len(),
            KeyValuesState::Owned(p) => p.len(),
        }
    }

    /// Check if there are no key-value pairs.
    pub const fn is_empty(&self) -> bool {
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

    /// Get the value for `key`, returning the last matching entry if duplicate keys exist.
    pub fn get(&self, key: &str) -> Option<ValueView<'a>> {
        match self.0 {
            KeyValuesState::Borrowed(p) => p
                .iter()
                .rev()
                .find_map(|(k, v)| (k.view().as_str() == key).then(|| v.view())),
            KeyValuesState::Owned(p) => p
                .iter()
                .rev()
                .find_map(|(k, v)| (k.view().as_str() == key).then(|| v.view())),
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
    type Item = (KeyView<'a>, ValueView<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            KeyValuesIterState::Borrowed(iter) => iter.next().map(|(k, v)| (k.view(), v.view())),
            KeyValuesIterState::Owned(iter) => iter.next().map(|(k, v)| (k.view(), v.view())),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            KeyValuesIterState::Borrowed(iter) => iter.size_hint(),
            KeyValuesIterState::Owned(iter) => iter.size_hint(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Record;

    #[test]
    fn duplicate_key_lookups_survive_ownership() {
        fn assert_traits<T: Copy + Send + Sync>() {}
        assert_traits::<Value<'_>>();
        let first = Value::from(&1_i32);
        assert_eq!(first.view().to_i64(), Some(1));
        assert!(matches!(Value::from(None::<i32>).view(), ValueView::None));
        let second = ValueOwned::i64(2);
        let key = Key::new("k");
        let inner = [(key, first), (key, Value::from(&second))];
        let outer = [(key, first), (key, Value::from(Some(Value::map(&inner))))];
        let record = Record::builder().key_values(outer.as_slice()).build();
        let check = |r: Record<'_>| {
            let map = r.key_values().get("k").unwrap().to_map().unwrap();
            assert_eq!(map.get("k").unwrap().to_i64(), Some(2));
        };
        let owned = record.to_owned();
        check(record);
        owned.with(check);
    }
}
