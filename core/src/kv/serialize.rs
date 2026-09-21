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

use std::collections::HashMap;
use std::fmt;

use super::KeyOwned;
use super::ValueOwned;
use super::ValueView;
use crate::Error;

/// Capture a serializable field in Logforth's native value model.
///
/// Nested structures and scalar types are preserved. If serialization fails,
/// the field becomes a string of the form `<serialization failed: ...>`; the
/// rest of the event is still emitted. Serialization errors do not cause a panic
/// or recursive logging. Panics in a custom serializer are not caught.
/// For explicit error handling, use [`ValueOwned::from_serde`] instead.
///
/// Call this inside a native macro to serialize only after metadata filtering:
///
/// ```
/// let logger = logforth_core::builder().build();
/// logforth_core::info!(logger, {
///     "samples" => logforth_core::kv::serde(&[1, 2, 3]),
/// }, "resource sample");
/// ```
pub fn serde(value: &(impl serde::Serialize + ?Sized)) -> ValueOwned {
    ValueOwned::from_serde(value)
        .unwrap_or_else(|err| ValueOwned::str(format!("<serialization failed: {err}>")))
}

impl ValueOwned {
    /// Serialize into Logforth's native value model.
    ///
    /// Numbers (including 128-bit integers), booleans, strings, bytes, lists, and
    /// nested maps retain their types. Map keys are converted to strings; keys
    /// that produce the same string overwrite earlier entries. Enums use Serde's
    /// externally tagged representation. Unit structs are represented by name.
    /// Returns an error if the source's serializer fails.
    ///
    /// Place the conversion inside a native macro's field expression to skip it
    /// when metadata filtering rejects the event. Handle failures explicitly;
    /// logging does not silently discard a failed field or substitute null.
    ///
    /// ```
    /// # fn main() -> Result<(), logforth_core::Error> {
    /// use logforth_core::kv::ValueOwned;
    /// let logger = logforth_core::builder().build();
    /// logforth_core::info!(logger, {
    ///     "samples" => ValueOwned::from_serde(&[1, 2, 3])?,
    /// }, "resource sample");
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_serde(value: &(impl serde::Serialize + ?Sized)) -> Result<Self, Error> {
        value
            .serialize(ValueSerializer)
            .map_err(|err| Error::new("failed to serialize log value").with_source(err))
    }
}

// Derived from the serializer previously implemented in logforth-bridge-log,
// based on opentelemetry-appender-log:
// https://github.com/open-telemetry/opentelemetry-rust/blob/f7b0dd99/opentelemetry-appender-log/src/lib.rs#L304-L763
struct ValueSerializer;

struct ValueSerializeSeq {
    value: Vec<ValueOwned>,
}

struct ValueSerializeTuple {
    value: Vec<ValueOwned>,
}

struct ValueSerializeTupleStruct {
    value: Vec<ValueOwned>,
}

struct ValueSerializeMap {
    key: Option<KeyOwned>,
    value: HashMap<KeyOwned, ValueOwned>,
}

struct ValueSerializeStruct {
    value: HashMap<KeyOwned, ValueOwned>,
}

struct ValueSerializeTupleVariant {
    variant: &'static str,
    value: Vec<ValueOwned>,
}

struct ValueSerializeStructVariant {
    variant: &'static str,
    value: HashMap<KeyOwned, ValueOwned>,
}

#[derive(Debug)]
struct ValueError(String);

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl serde::ser::Error for ValueError {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        ValueError(msg.to_string())
    }
}

impl std::error::Error for ValueError {}

impl serde::Serializer for ValueSerializer {
    type Ok = ValueOwned;

    type Error = ValueError;

    type SerializeSeq = ValueSerializeSeq;

    type SerializeTuple = ValueSerializeTuple;

    type SerializeTupleStruct = ValueSerializeTupleStruct;

    type SerializeTupleVariant = ValueSerializeTupleVariant;

    type SerializeMap = ValueSerializeMap;

    type SerializeStruct = ValueSerializeStruct;

    type SerializeStructVariant = ValueSerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::i64(v))
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::i128(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::u64(v))
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::u128(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::f64(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::char(v))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::str(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::bytes(v.to_vec()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::none())
    }

    fn serialize_some<T: serde::Serialize + ?Sized>(
        self,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::none())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::str(name))
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::str(variant))
    }

    fn serialize_newtype_struct<T: serde::Serialize + ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: serde::Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        let mut map = self.serialize_map(Some(1))?;
        serde::ser::SerializeMap::serialize_entry(&mut map, variant, value)?;
        serde::ser::SerializeMap::end(map)
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(ValueSerializeSeq { value: vec![] })
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(ValueSerializeTuple { value: vec![] })
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(ValueSerializeTupleStruct { value: vec![] })
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(ValueSerializeTupleVariant {
            variant,
            value: vec![],
        })
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(ValueSerializeMap {
            key: None,
            value: HashMap::new(),
        })
    }

    fn serialize_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(ValueSerializeStruct {
            value: HashMap::new(),
        })
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(ValueSerializeStructVariant {
            variant,
            value: HashMap::new(),
        })
    }
}

impl serde::ser::SerializeSeq for ValueSerializeSeq {
    type Ok = ValueOwned;

    type Error = ValueError;

    fn serialize_element<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.value.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::from_vec(self.value))
    }
}

impl serde::ser::SerializeTuple for ValueSerializeTuple {
    type Ok = ValueOwned;

    type Error = ValueError;

    fn serialize_element<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.value.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::from_vec(self.value))
    }
}

impl serde::ser::SerializeTupleStruct for ValueSerializeTupleStruct {
    type Ok = ValueOwned;

    type Error = ValueError;

    fn serialize_field<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.value.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::from_vec(self.value))
    }
}

impl serde::ser::SerializeTupleVariant for ValueSerializeTupleVariant {
    type Ok = ValueOwned;

    type Error = ValueError;

    fn serialize_field<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.value.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::from_hash_map({
            let mut variant = HashMap::<KeyOwned, ValueOwned>::new();
            variant.insert(KeyOwned::new(self.variant), ValueOwned::list(self.value));
            variant
        }))
    }
}

impl serde::ser::SerializeMap for ValueSerializeMap {
    type Ok = ValueOwned;

    type Error = ValueError;

    fn serialize_key<T: serde::Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
        let key = match key.serialize(ValueSerializer)?.view() {
            ValueView::StaticStr(s) => KeyOwned::new(s),
            value => KeyOwned::new(value.to_string()),
        };
        self.key = Some(key);
        Ok(())
    }

    fn serialize_value<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        let key = self
            .key
            .take()
            .ok_or_else(|| serde::ser::Error::custom("missing key"))?;
        let value = value.serialize(ValueSerializer)?;
        self.value.insert(key, value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::from_hash_map(self.value))
    }
}

impl serde::ser::SerializeStruct for ValueSerializeStruct {
    type Ok = ValueOwned;

    type Error = ValueError;

    fn serialize_field<T: serde::Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        let key = KeyOwned::new(key);
        let value = value.serialize(ValueSerializer)?;
        self.value.insert(key, value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::from_hash_map(self.value))
    }
}

impl serde::ser::SerializeStructVariant for ValueSerializeStructVariant {
    type Ok = ValueOwned;

    type Error = ValueError;

    fn serialize_field<T: serde::Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        let key = KeyOwned::new(key);
        let value = value.serialize(ValueSerializer)?;
        self.value.insert(key, value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::from_hash_map({
            let mut variant = HashMap::<KeyOwned, ValueOwned>::new();
            variant.insert(
                KeyOwned::new(self.variant),
                ValueOwned::from_hash_map(self.value),
            );
            variant
        }))
    }
}
