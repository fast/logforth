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

use super::Value;
use super::ValueOwned;

impl<'a> From<&&'a str> for Value<'a> {
    fn from(value: &&'a str) -> Self {
        Self::str(value)
    }
}

impl<'a> From<&'a Value<'_>> for Value<'a> {
    fn from(value: &'a Value<'_>) -> Self {
        *value
    }
}

impl<'a> From<&'a ValueOwned> for Value<'a> {
    fn from(value: &'a ValueOwned) -> Self {
        Self::borrowed(value)
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(value: &'a str) -> Self {
        Self::str(value)
    }
}

impl<'a> From<&'a String> for Value<'a> {
    fn from(value: &'a String) -> Self {
        Self::str(value)
    }
}

impl<'a> From<&&'a String> for Value<'a> {
    fn from(value: &&'a String) -> Self {
        Self::str(value)
    }
}

impl<'a> From<&&'a Value<'_>> for Value<'a> {
    fn from(value: &&'a Value<'_>) -> Self {
        **value
    }
}

impl<'a> From<&&'a ValueOwned> for Value<'a> {
    fn from(value: &&'a ValueOwned) -> Self {
        Self::borrowed(value)
    }
}

macro_rules! scalar {
    ($constructor:ident, $repr:ty, $($ty:ty),+ $(,)?) => {
        $(impl From<$ty> for Value<'_> {
            fn from(value: $ty) -> Self {
                Self::$constructor(value as $repr)
            }
        }

        impl From<&$ty> for Value<'_> {
            fn from(value: &$ty) -> Self {
                <Self as From<$ty>>::from(*value)
            }
        }

        impl From<&&$ty> for Value<'_> {
            fn from(value: &&$ty) -> Self {
                <Self as From<$ty>>::from(**value)
            }
        })+
    };
}

scalar!(bool, bool, bool);
scalar!(char, char, char);
scalar!(i64, i64, i8, i16, i32, i64, isize);
scalar!(u64, u64, u8, u16, u32, u64, usize);
scalar!(i128, i128, i128);
scalar!(u128, u128, u128);
scalar!(f64, f64, f32, f64);
