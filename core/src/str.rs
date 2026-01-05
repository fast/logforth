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

    pub fn into_cow_static(self) -> Cow<'static, str> {
        match self {
            RefStr::Borrowed(s) => Cow::Owned(ToOwned::to_owned(s)),
            RefStr::Static(s) => Cow::Borrowed(s),
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
