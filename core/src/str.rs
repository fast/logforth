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
use std::cmp::Ordering;
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub enum RefStr<'a> {
    Borrowed(&'a str),
    Static(&'static str),
}

impl fmt::Debug for RefStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.get(), f)
    }
}

impl fmt::Display for RefStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.get(), f)
    }
}

impl<'a> RefStr<'a> {
    pub const fn get(&self) -> &'a str {
        match self {
            RefStr::Borrowed(s) => s,
            RefStr::Static(s) => s,
        }
    }

    pub const fn get_static(&self) -> Option<&'static str> {
        match self {
            RefStr::Borrowed(_) => None,
            RefStr::Static(s) => Some(s),
        }
    }

    pub fn to_cow_static(&self) -> Cow<'static, str> {
        match self {
            RefStr::Borrowed(s) => Cow::Owned(ToOwned::to_owned(*s)),
            RefStr::Static(s) => Cow::Borrowed(s),
        }
    }

    pub fn to_owned(&self) -> OwnedStr {
        match self {
            RefStr::Borrowed(s) => OwnedStr::Owned(Box::from(*s)),
            RefStr::Static(s) => OwnedStr::Static(s),
        }
    }
}

impl PartialEq for RefStr<'_> {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(self.get(), other.get())
    }
}

impl Eq for RefStr<'_> {}

impl PartialOrd for RefStr<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RefStr<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(self.get(), other.get())
    }
}

impl Hash for RefStr<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Hash::hash(self.get(), state)
    }
}

#[derive(Clone)]
pub enum OwnedStr {
    Owned(Box<str>),
    Static(&'static str),
    Shared(Arc<str>),
}

impl fmt::Debug for OwnedStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.get(), f)
    }
}

impl fmt::Display for OwnedStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.get(), f)
    }
}


impl OwnedStr {
    pub fn get(&self) -> &str {
        match self {
            OwnedStr::Owned(s) => s,
            OwnedStr::Static(s) => s,
            OwnedStr::Shared(s) => s,
        }
    }

    pub fn get_static(&self) -> Option<&'static str> {
        match self {
            OwnedStr::Owned(_) | OwnedStr::Shared(_) => None,
            OwnedStr::Static(s) => Some(s),
        }
    }

    pub fn by_ref(&self) -> RefStr<'_> {
        match self {
            OwnedStr::Owned(s) => RefStr::Borrowed(s),
            OwnedStr::Static(s) => RefStr::Static(s),
            OwnedStr::Shared(s) => RefStr::Borrowed(s),
        }
    }
}

impl PartialEq for OwnedStr {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(self.get(), other.get())
    }
}

impl Eq for OwnedStr {}

impl PartialOrd for OwnedStr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OwnedStr {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(self.get(), other.get())
    }
}

impl Hash for OwnedStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Hash::hash(self.get(), state)
    }
}
