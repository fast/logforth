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

use std::fmt;

use crate::Error;
use crate::kv::KeyOwned;
use crate::kv::ValueOwned;
use crate::kv::ValueOwnedState;

impl ValueOwned {
    /// Serializes a [`serde::Serialize`] value into an owned structured [`ValueOwned`].
    ///
    /// If serialization fails, returns a string placeholder describing the failure instead of
    /// panicking.
    #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
    pub fn from_serde(value: &(impl serde::Serialize + ?Sized)) -> Self {
        Self::try_from_serde(value)
            .unwrap_or_else(|err| Self::str(format!("<serde error: {}>", err.message())))
    }

    /// Attempts to serialize a [`serde::Serialize`] value into an owned structured [`ValueOwned`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if `value`'s [`serde::Serialize`] implementation fails.
    #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
    pub fn try_from_serde(value: &(impl serde::Serialize + ?Sized)) -> Result<Self, Error> {
        value.serialize(ValueSerializer).map_err(|err| err.0)
    }
}

#[derive(Debug)]
struct ValueError(Error);

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for ValueError {}

impl serde::ser::Error for ValueError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self(Error::new(msg.to_string()))
    }
}

struct ValueSerializer;

struct ValueSerializeSeq {
    values: Vec<ValueOwned>,
}

struct ValueSerializeTupleVariant {
    variant: &'static str,
    values: Vec<ValueOwned>,
}

struct ValueSerializeMap {
    key: Option<KeyOwned>,
    entries: Vec<(KeyOwned, ValueOwned)>,
}

struct ValueSerializeStruct {
    entries: Vec<(KeyOwned, ValueOwned)>,
}

struct ValueSerializeStructVariant {
    variant: &'static str,
    entries: Vec<(KeyOwned, ValueOwned)>,
}

impl serde::Serializer for ValueSerializer {
    type Ok = ValueOwned;
    type Error = ValueError;
    type SerializeSeq = ValueSerializeSeq;
    type SerializeTuple = ValueSerializeSeq;
    type SerializeTupleStruct = ValueSerializeSeq;
    type SerializeTupleVariant = ValueSerializeTupleVariant;
    type SerializeMap = ValueSerializeMap;
    type SerializeStruct = ValueSerializeStruct;
    type SerializeStructVariant = ValueSerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::i64(i64::from(v)))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::i64(i64::from(v)))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::i64(i64::from(v)))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::i64(v))
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::i128(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::u64(u64::from(v)))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::u64(u64::from(v)))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::u64(u64::from(v)))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::u64(v))
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::u128(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::f64(f64::from(v)))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::f64(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::char(v))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::str(v.to_owned()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::bytes(v))
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
        Ok(ValueOwned::map([(
            KeyOwned::new(variant),
            value.serialize(ValueSerializer)?,
        )]))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(ValueSerializeSeq {
            values: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(ValueSerializeSeq {
            values: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(ValueSerializeSeq {
            values: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(ValueSerializeTupleVariant {
            variant,
            values: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(ValueSerializeMap {
            key: None,
            entries: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_struct(
        self,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(ValueSerializeStruct {
            entries: Vec::with_capacity(len),
        })
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(ValueSerializeStructVariant {
            variant,
            entries: Vec::with_capacity(len),
        })
    }

    fn collect_str<T: ?Sized + fmt::Display>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::str(value.to_string()))
    }
}

impl serde::ser::SerializeSeq for ValueSerializeSeq {
    type Ok = ValueOwned;
    type Error = ValueError;

    fn serialize_element<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.values.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::list(self.values))
    }
}

impl serde::ser::SerializeTuple for ValueSerializeSeq {
    type Ok = ValueOwned;
    type Error = ValueError;

    fn serialize_element<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.values.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::list(self.values))
    }
}

impl serde::ser::SerializeTupleStruct for ValueSerializeSeq {
    type Ok = ValueOwned;
    type Error = ValueError;

    fn serialize_field<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.values.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::list(self.values))
    }
}

impl serde::ser::SerializeTupleVariant for ValueSerializeTupleVariant {
    type Ok = ValueOwned;
    type Error = ValueError;

    fn serialize_field<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.values.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::map([(
            KeyOwned::new(self.variant),
            ValueOwned::list(self.values),
        )]))
    }
}

impl serde::ser::SerializeMap for ValueSerializeMap {
    type Ok = ValueOwned;
    type Error = ValueError;

    fn serialize_key<T: serde::Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
        let key = match key.serialize(ValueSerializer)?.0 {
            ValueOwnedState::Str(s) => KeyOwned(s),
            other => KeyOwned::new(ValueOwned(other).view().to_string()),
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
            .ok_or_else(|| serde::ser::Error::custom("missing map key"))?;
        self.entries.push((key, value.serialize(ValueSerializer)?));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::map(self.entries))
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
        self.entries
            .push((KeyOwned::new(key), value.serialize(ValueSerializer)?));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::map(self.entries))
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
        self.entries
            .push((KeyOwned::new(key), value.serialize(ValueSerializer)?));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueOwned::map([(
            KeyOwned::new(self.variant),
            ValueOwned::map(self.entries),
        )]))
    }
}
