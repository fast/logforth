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

//! The [`Str`] type.
//!
//! This module implements a string type that combines `Cow<'static, str>` with `Cow<'a, str>`. A
//! [`Str`] can hold borrowed, static, or shared data. Internally, it's more efficient than a
//! [`Cow`] to access because it doesn't need to hop through enum variants.

// This file is derived from https://github.com/emit-rs/emit/blob/097f5254/core/src/str.rs

use std::borrow::Borrow;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;
use std::hash;
use std::marker::PhantomData;
use std::sync::Arc;

/// A string value.
pub struct Str<'k> {
    // This type is an optimized `Cow<str>`
    // It avoids the cost of matching the variant to get the inner value
    value: *const str,
    owner: StrOwner,
    marker: PhantomData<&'k str>,
}

// SAFETY: `Str` synchronizes through `Arc` when ownership is shared
unsafe impl<'k> Send for Str<'k> {}
// SAFETY: `Str` does not use interior mutability
unsafe impl<'k> Sync for Str<'k> {}

impl<'k> Default for Str<'k> {
    fn default() -> Self {
        Str::new(Default::default())
    }
}

enum StrOwner {
    None,
    Static(&'static str),
    Shared(Arc<str>),
}

impl<'k> fmt::Debug for Str<'k> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.get(), f)
    }
}

impl<'k> fmt::Display for Str<'k> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.get(), f)
    }
}

impl<'k> Clone for Str<'k> {
    fn clone(&self) -> Self {
        match self.owner {
            StrOwner::Shared(ref value) => Str::new_shared(value.clone()),
            StrOwner::Static(owner) => Str {
                value: self.value,
                owner: StrOwner::Static(owner),
                marker: PhantomData,
            },
            StrOwner::None => Str {
                value: self.value,
                owner: StrOwner::None,
                marker: PhantomData,
            },
        }
    }
}

impl Str<'static> {
    /// Create a new string from a value borrowed for `'static`.
    pub const fn new(k: &'static str) -> Self {
        Str {
            value: k as *const str,
            owner: StrOwner::Static(k),
            marker: PhantomData,
        }
    }

    /// Create a string from a shared value.
    ///
    /// Cloning the string will involve cloning the `Arc`, which may be cheaper than cloning the
    /// value itself.
    pub fn new_shared(key: impl Into<Arc<str>>) -> Self {
        let value = key.into();

        Str {
            value: &*value as *const str,
            owner: StrOwner::Shared(value),
            marker: PhantomData,
        }
    }
}

impl<'k> Str<'k> {
    /// Create a new string from a value borrowed for `'k`.
    ///
    /// The [`Str::new`] method should be preferred where possible.
    pub const fn new_ref(k: &'k str) -> Str<'k> {
        Str {
            value: k as *const str,
            owner: StrOwner::None,
            marker: PhantomData,
        }
    }

    /// Get a new [`Str`], borrowing data from this one.
    pub const fn by_ref(&self) -> Str<'_> {
        Str {
            value: self.value,
            owner: match self.owner {
                StrOwner::Static(owner) => StrOwner::Static(owner),
                StrOwner::None | StrOwner::Shared(_) => StrOwner::None,
            },
            marker: PhantomData,
        }
    }

    /// Get a reference to the underlying value.
    pub const fn get(&self) -> &str {
        // NOTE: It's important here that the lifetime returned is not `'k`
        // If it was it would be possible to return a `&'static str` from
        // an owned value
        // SAFETY: `self.value` is guaranteed to outlive the borrow of `self`
        unsafe { &(*self.value) }
    }

    /// Try to get a reference to the underlying static value.
    ///
    /// If the string was created from [`Str::new`] and contains a `'static` value then this method
    /// will return `Some`. Otherwise, this method will return `None`.
    pub const fn get_static(&self) -> Option<&'static str> {
        if let StrOwner::Static(owner) = self.owner {
            Some(owner)
        } else {
            None
        }
    }

    /// Get a new string, taking an owned copy of the data in this one.
    ///
    /// If the string contains a `'static` or `Arc` value then this method is cheap. In other cases
    /// the underlying value will be passed through [`Str::new_shared`].
    pub fn to_shared(&self) -> Str<'static> {
        match self.owner {
            StrOwner::Static(owner) => Str::new(owner),
            StrOwner::Shared(ref owner) => Str::new_shared(owner.clone()),
            StrOwner::None => Str::new_shared(self.get()),
        }
    }

    /// Get the underlying value as a potentially owned string.
    ///
    /// If the string contains a `'static` value then this method will return `Cow::Borrowed`.
    /// Otherwise, it will return `Cow::Owned`.
    pub fn to_cow(&self) -> Cow<'static, str> {
        match self.owner {
            StrOwner::Static(key) => Cow::Borrowed(key),
            StrOwner::None | StrOwner::Shared(_) => Cow::Owned(self.get().to_owned()),
        }
    }
}

impl<'a> hash::Hash for Str<'a> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state)
    }
}

impl std::ops::Deref for Str<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<'a, 'b> PartialEq<Str<'b>> for Str<'a> {
    fn eq(&self, other: &Str<'b>) -> bool {
        self.get() == other.get()
    }
}

impl<'a> Eq for Str<'a> {}

impl<'a> PartialEq<str> for Str<'a> {
    fn eq(&self, other: &str) -> bool {
        self.get() == other
    }
}

impl<'a> PartialEq<Str<'a>> for str {
    fn eq(&self, other: &Str<'a>) -> bool {
        self == other.get()
    }
}

impl<'a, 'b> PartialEq<&'b str> for Str<'a> {
    fn eq(&self, other: &&'b str) -> bool {
        self.get() == *other
    }
}

impl<'b> PartialEq<Str<'b>> for &str {
    fn eq(&self, other: &Str<'b>) -> bool {
        *self == other.get()
    }
}

impl<'a, 'b> PartialOrd<Str<'b>> for Str<'a> {
    fn partial_cmp(&self, other: &Str<'b>) -> Option<Ordering> {
        self.get().partial_cmp(other.get())
    }
}

impl<'a> Ord for Str<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.get().cmp(other.get())
    }
}

impl<'k> Borrow<str> for Str<'k> {
    fn borrow(&self) -> &str {
        self.get()
    }
}

impl<'k> AsRef<str> for Str<'k> {
    fn as_ref(&self) -> &str {
        self.get()
    }
}
