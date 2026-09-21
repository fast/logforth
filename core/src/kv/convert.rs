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

/// Borrow a typed value for a structured log field.
///
/// Native macros borrow their arguments through this trait, so strings and owned
/// values are not consumed. Implement it for domain types with a natural scalar
/// representation. Use [`Value::debug`] or [`Value::display`] for text, and
/// `ValueOwned::from_serde` (with the `serde` feature) for nested structures.
pub trait ToValue {
    /// Borrow this value, preserving its supported scalar or structured type.
    fn to_value(&self) -> Value<'_>;
}

impl<T: ToValue + ?Sized> ToValue for &T {
    fn to_value(&self) -> Value<'_> {
        T::to_value(self)
    }
}

impl ToValue for Value<'_> {
    fn to_value(&self) -> Value<'_> {
        *self
    }
}

impl ToValue for ValueOwned {
    fn to_value(&self) -> Value<'_> {
        Value::borrowed(self)
    }
}

impl ToValue for str {
    fn to_value(&self) -> Value<'_> {
        Value::str(self)
    }
}

impl ToValue for String {
    fn to_value(&self) -> Value<'_> {
        Value::str(self)
    }
}

impl<T: ToValue> ToValue for Option<T> {
    fn to_value(&self) -> Value<'_> {
        self.as_ref().map_or_else(Value::none, ToValue::to_value)
    }
}

macro_rules! scalar {
    ($constructor:ident, $repr:ty, $($ty:ty),+ $(,)?) => {
        $(impl ToValue for $ty {
            fn to_value(&self) -> Value<'_> {
                Value::$constructor(*self as $repr)
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
