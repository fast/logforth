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
//! [`Str`] can hold borrowed, static, owned, or shared data. Internally, it's more efficient than a
//! [`Cow`] to access because it doesn't need to hop through enum variants.
//!
//! Values can be converted into [`Str`]s either directly using methods like [`Str::new`], or
//! generically through the [`ToStr`] trait.

// This file is derived from https://github.com/emit-rs/emit/blob/097f5254/core/src/str.rs

use std::borrow::Borrow;
use std::borrow::Cow;
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

enum StrOwner {
    None,
    Static(&'static str),
    Box(*mut str),
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
            StrOwner::Box(_) => Str::new_owned(unsafe { &*self.value }),
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

impl<'k> Drop for Str<'k> {
    fn drop(&mut self) {
        match self.owner {
            StrOwner::Box(boxed) => drop(unsafe { Box::from_raw(boxed) }),
            // Other cases handled normally
            _ => (),
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

    /// Create a string from an owned value.
    ///
    /// Cloning the string will involve cloning the value.
    pub fn new_owned(key: impl Into<Box<str>>) -> Self {
        let value = key.into();

        let raw = Box::into_raw(value);

        Str {
            value: raw as *const str,
            owner: StrOwner::Box(raw),
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

    /// Create a string from a potentially owned value.
    ///
    /// If the value is `Cow::Borrowed` then this method will defer to [`Str::new_ref`]. If the
    /// value is `Cow::Owned` then this method will defer to [`Str::new_owned`].
    pub fn new_cow_ref(key: Cow<'k, str>) -> Self {
        match key {
            Cow::Borrowed(key) => Str::new_ref(key),
            Cow::Owned(key) => Str::new_owned(key),
        }
    }

    /// Get a new string, borrowing data from this one.
    pub const fn by_ref(&self) -> Str<'_> {
        Str {
            value: self.value,
            owner: match self.owner {
                StrOwner::Static(owner) => StrOwner::Static(owner),
                _ => StrOwner::None,
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

    /// Get the underlying value as a potentially owned string.
    ///
    /// If the string contains a `'static` value then this method will return `Cow::Borrowed`.
    /// Otherwise, it will return `Cow::Owned`.
    pub fn to_cow(&self) -> Cow<'static, str> {
        match self.owner {
            StrOwner::Static(key) => Cow::Borrowed(key),
            _ => Cow::Owned(self.get().to_owned()),
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
            _ => Str::new_shared(self.get()),
        }
    }

    /// Get a new string, taking an owned copy of the data in this one.
    ///
    /// If the string contains a `'static` or `Arc` value then this method is cheap and doesn't
    /// involve cloning. In other cases the underlying value will be passed through
    /// [`Str::new_owned`].
    pub fn to_owned(&self) -> Str<'static> {
        match self.owner {
            StrOwner::Static(owner) => Str::new(owner),
            StrOwner::Shared(ref owner) => Str::new_shared(owner.clone()),
            _ => Str::new_owned(self.get()),
        }
    }

    /// Convert this string into an owned `String`.
    ///
    /// If the underlying value is already an owned string then this method will return it without
    /// allocating.
    pub fn into_string(self) -> String {
        match self.owner {
            StrOwner::Box(boxed) => {
                // Ensure `Drop` doesn't run over this value
                // and clean up the box we've just moved out of
                std::mem::forget(self);
                unsafe { Box::from_raw(boxed) }.into()
            }
            _ => self.get().to_owned(),
        }
    }
}

impl<'a> hash::Hash for Str<'a> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state)
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
    fn partial_cmp(&self, other: &Str<'b>) -> Option<core::cmp::Ordering> {
        self.get().partial_cmp(other.get())
    }
}

impl<'a> Ord for Str<'a> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
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

/// Convert a reference to a [`Str`].
pub trait ToStr {
    /// Perform the conversion.
    fn to_str(&self) -> Str<'_>;
}

impl<'a, T: ToStr + ?Sized> ToStr for &'a T {
    fn to_str(&self) -> Str<'_> {
        (**self).to_str()
    }
}

impl<'k> ToStr for Str<'k> {
    fn to_str(&self) -> Str<'_> {
        self.by_ref()
    }
}

impl ToStr for str {
    fn to_str(&self) -> Str<'_> {
        Str::new_ref(self)
    }
}

impl ToStr for String {
    fn to_str(&self) -> Str<'_> {
        Str::new_ref(self)
    }
}

impl ToStr for Box<str> {
    fn to_str(&self) -> Str<'_> {
        Str::new_ref(self)
    }
}

impl ToStr for Arc<str> {
    fn to_str(&self) -> Str<'_> {
        Str::new_shared(self.clone())
    }
}

impl From<String> for Str<'static> {
    fn from(value: String) -> Self {
        Str::new_owned(value)
    }
}

impl From<Box<str>> for Str<'static> {
    fn from(value: Box<str>) -> Self {
        Str::new_owned(value)
    }
}

impl From<Arc<str>> for Str<'static> {
    fn from(value: Arc<str>) -> Self {
        Str::new_shared(value)
    }
}

impl<'k> From<&'k String> for Str<'k> {
    fn from(value: &'k String) -> Self {
        Str::new_ref(value)
    }
}

impl<'k> From<Str<'k>> for String {
    fn from(value: Str<'k>) -> String {
        value.into_string()
    }
}

impl<'a> From<&'a str> for Str<'a> {
    fn from(value: &'a str) -> Self {
        Str::new_ref(value)
    }
}

impl<'a, 'b> From<&'a Str<'b>> for Str<'a> {
    fn from(value: &'a Str<'b>) -> Self {
        value.by_ref()
    }
}
