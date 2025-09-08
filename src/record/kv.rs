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
use std::fmt;
use std::slice;

use value_bag::OwnedValueBag;
use value_bag::ValueBag;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KeyInner<'a> {
    Str(&'a str),
    StaticStr(&'static str),
}

/// Represents a key in a key-value pair.
#[derive(Debug, Clone)]
pub struct Key<'a>(KeyInner<'a>);

impl Key<'_> {
    /// Gets the key string.
    pub fn as_str(&self) -> &str {
        match &self.0 {
            KeyInner::Str(s) => s,
            KeyInner::StaticStr(s) => s,
        }
    }
}

impl<'a> Key<'a> {
    #[doc(hidden)]
    pub fn __from_static_str(key: &'static str) -> Self {
        Key(KeyInner::StaticStr(key))
    }

    pub(crate) fn from_str(key: &'a str) -> Self {
        Key(KeyInner::Str(key))
    }

    pub(crate) fn to_owned(&self) -> KeyOwned {
        let inner = match self.0 {
            KeyInner::Str(s) => KeyOwnedInner::CowStr(Cow::Owned(s.to_string())),
            KeyInner::StaticStr(s) => KeyOwnedInner::CowStr(Cow::Borrowed(s)),
        };
        KeyOwned(inner)
    }

    #[cfg(test)]
    pub(crate) fn inner(&self) -> KeyInner<'a> {
        self.0.clone()
    }
}

impl PartialEq for Key<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

#[derive(Debug, Clone)]
enum KeyOwnedInner {
    CowStr(Cow<'static, str>),
}

#[derive(Debug, Clone)]
pub(crate) struct KeyOwned(KeyOwnedInner);

impl KeyOwned {
    pub(crate) fn as_ref(&self) -> Key<'_> {
        let inner = match &self.0 {
            KeyOwnedInner::CowStr(s) => match s {
                Cow::Borrowed(s) => KeyInner::StaticStr(s),
                Cow::Owned(s) => KeyInner::Str(s),
            },
        };
        Key(inner)
    }
}

/// Represents a value in a key-value pair.
pub type Value<'a> = ValueBag<'a>;
pub(crate) type ValueOwned = OwnedValueBag;

enum KeyValuesInner<'a> {
    Borrowed(&'a [Pair<'a>]),
    Owned(&'a [(KeyOwned, ValueOwned)]),
}
enum KeyValuesIterInner<'a> {
    Borrowed(slice::Iter<'a, Pair<'a>>),
    Owned(slice::Iter<'a, (KeyOwned, ValueOwned)>),
}

/// Represents a collection of key-value pairs.
pub struct KeyValues<'a>(KeyValuesInner<'a>);

impl<'a> KeyValues<'a> {
    /// Gets the number of key-value pairs.
    pub fn len(&self) -> usize {
        match self.0 {
            KeyValuesInner::Borrowed(p) => p.len(),
            KeyValuesInner::Owned(p) => p.len(),
        }
    }

    /// Checks if there are no key-value pairs.
    pub fn is_empty(&self) -> bool {
        match self.0 {
            KeyValuesInner::Borrowed(p) => p.is_empty(),
            KeyValuesInner::Owned(p) => p.is_empty(),
        }
    }

    /// Gets the value of the specified key.
    pub fn get(&self, key: Key) -> Option<Value<'a>> {
        match self.0 {
            KeyValuesInner::Borrowed(p) => p
                .iter()
                .find_map(|(k, v)| if k == &key { Some(v.clone()) } else { None }),
            KeyValuesInner::Owned(p) => p.iter().find_map(|(k, v)| {
                if k.as_ref() == key {
                    Some(v.by_ref())
                } else {
                    None
                }
            }),
        }
    }

    /// Gets an iterator over the key-value pairs.
    pub fn iter(&self) -> KeyValuesIter<'a> {
        match &self.0 {
            KeyValuesInner::Borrowed(p) => KeyValuesIter(KeyValuesIterInner::Borrowed(p.iter())),
            KeyValuesInner::Owned(p) => KeyValuesIter(KeyValuesIterInner::Owned(p.iter())),
        }
    }

    pub(crate) fn with_borrowed(pairs: &'a [Pair<'a>]) -> Self {
        Self(KeyValuesInner::Borrowed(pairs))
    }

    pub(crate) fn with_owned(pairs: &'a [(KeyOwned, ValueOwned)]) -> Self {
        Self(KeyValuesInner::Owned(pairs))
    }

    pub(crate) fn write_to(&self, dest: &mut impl fmt::Write, brackets: bool) -> fmt::Result {
        let mut iter = self.iter();
        let first = iter.next();
        if let Some((key, value)) = first {
            if brackets {
                dest.write_str(" { ")?;
            }

            // Reduce branch prediction misses for performance
            // So we manually process the first KV pair
            dest.write_str(key.as_str())?;
            dest.write_str("=")?;
            write!(dest, "{value}")?;

            for (key, value) in iter {
                dest.write_str(" ")?;
                dest.write_str(key.as_str())?;
                dest.write_str("=")?;
                write!(dest, "{value}")?;
            }

            if brackets {
                dest.write_str(" }")?;
            }
        }
        Ok(())
    }
}

impl<'a> IntoIterator for KeyValues<'a> {
    type Item = Pair<'a>;
    type IntoIter = KeyValuesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Represents an iterator over key-value pairs.
pub struct KeyValuesIter<'a>(KeyValuesIterInner<'a>);

impl<'a> Iterator for KeyValuesIter<'a> {
    type Item = Pair<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            // The 2 clones should be cheap
            KeyValuesIterInner::Borrowed(iter) => iter.next().map(|(k, v)| (k.clone(), v.clone())),
            KeyValuesIterInner::Owned(iter) => iter.next().map(|(k, v)| (k.as_ref(), v.by_ref())),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            KeyValuesIterInner::Borrowed(iter) => iter.size_hint(),
            KeyValuesIterInner::Owned(iter) => iter.size_hint(),
        }
    }
}

pub(crate) type Pair<'a> = (Key<'a>, Value<'a>);

pub(crate) struct LogCrateConverter<'a>(Vec<(log::kv::Key<'a>, ValueOwned)>);

impl<'a> LogCrateConverter<'a> {
    pub(crate) fn new(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub(crate) fn finalize(self) -> Vec<(log::kv::Key<'a>, ValueOwned)> {
        self.0
    }
}

impl<'a> log::kv::VisitSource<'a> for LogCrateConverter<'a> {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'a>,
        value: log::kv::Value<'a>,
    ) -> Result<(), log::kv::Error> {
        struct Visitor(Option<ValueOwned>);

        macro_rules! visit_fn {
            ( $($fn:ident: $ty:ty => $from:ident),+$(,)? ) => {
                $(fn $fn(&mut self, value: $ty) -> Result<(), log::kv::Error> {
                    self.0 = Some(Value::$from(value).to_owned());
                    Ok(())
                })+
            };
        }

        impl log::kv::VisitValue<'_> for Visitor {
            fn visit_any(&mut self, value: log::kv::Value) -> Result<(), log::kv::Error> {
                // Since we have no way to extract the underlying `&dyn Display`, we have to
                // `to_owned()` here
                self.0 = Some(Value::from_display(&value).to_owned());
                Ok(())
            }

            fn visit_null(&mut self) -> Result<(), log::kv::Error> {
                self.0 = Some(Value::empty().to_owned());
                Ok(())
            }

            visit_fn! {
                visit_u64: u64 => from_u64,
                visit_i64: i64 => from_i64,
                visit_u128: u128 => from_u128,
                visit_i128: i128 => from_i128,
                visit_f64: f64 => from_f64,
                visit_bool: bool => from_bool,
                visit_str: &str => from_str,
                visit_borrowed_str: &str => from_str,
                visit_char: char => from_char,
            }
        }

        let mut visitor = Visitor(None);
        value.visit(&mut visitor)?;
        self.0.push((key, visitor.0.unwrap()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_partial_eq() {
        assert_eq!(Key::from_str("a"), Key::__from_static_str("a"));
        assert_ne!(Key::from_str("a"), Key::__from_static_str("b"));
    }
}
