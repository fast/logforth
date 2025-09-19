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

//! The module for key-value pairs in a log record.

use crate::Str;

use value_bag::OwnedValueBag;
use value_bag::ValueBag;

/// Represents a value in a key-value pair.
pub type Value<'a> = ValueBag<'a>;

/// Represents a key in a key-value pair.
#[derive(Debug, Clone)]
pub struct Key<'a>(Str<'a>);

impl Key<'_> {
    /// Gets the key string.
    pub fn as_str(&self) -> &str {
        self.0.get()
    }
}

impl<'a> Key<'a> {
    pub(crate) fn from_str(key: &'a str) -> Self {
        Key(Str::new_ref(key))
    }

    pub(crate) fn to_owned(&self) -> KeyOwned {
        KeyOwned(self.0.to_owned())
    }
}

impl PartialEq for Key<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

pub(crate) type ValueOwned = OwnedValueBag;

#[derive(Debug, Clone)]
pub(crate) struct KeyOwned(Str<'static>);

impl KeyOwned {
    pub(crate) fn as_ref(&self) -> Key<'_> {
        Key(self.0.by_ref())
    }
}
