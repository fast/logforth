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

//! A bridge to forward logs from the `log` crate to `logforth`.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

use std::ops::Deref;
use std::sync::Arc;

use log::Metadata;
use log::Record;
use logforth_core::Logger;
use logforth_core::record::FilterCriteria;

/// Adapter to use a `logforth` logger instance as a `log` crate logger.
#[derive(Debug)]
pub struct LogAdapter {
    logger: Arc<Logger>,
}

impl LogAdapter {
    /// Create a new `LogAdapter` instance.
    pub fn new(logger: impl Into<Arc<Logger>>) -> Self {
        Self {
            logger: logger.into(),
        }
    }
}

impl Deref for LogAdapter {
    type Target = Logger;

    fn deref(&self) -> &Self::Target {
        &self.logger
    }
}

impl log::Log for LogAdapter {
    fn enabled(&self, metadata: &Metadata) -> bool {
        forward_enabled(&self.logger, metadata)
    }

    fn log(&self, record: &Record) {
        forward_log(&self.logger, record);
    }

    fn flush(&self) {
        self.logger.flush();
    }
}

fn forward_enabled(logger: &Logger, metadata: &Metadata) -> bool {
    let criteria = FilterCriteria::builder()
        .target(metadata.target())
        .level(level_to_level(metadata.level()))
        .build();

    Logger::enabled(logger, &criteria)
}

fn forward_log(logger: &Logger, record: &Record) {
    if !forward_enabled(logger, record.metadata()) {
        return;
    }

    // basic fields
    let mut builder = logforth_core::record::Record::builder()
        .level(level_to_level(record.level()))
        .target(record.target())
        .line(record.line());

    // optional static fields
    builder = if let Some(module_path) = record.module_path_static() {
        builder.module_path_static(module_path)
    } else {
        builder.module_path(record.module_path())
    };
    builder = if let Some(file) = record.file_static() {
        builder.file_static(file)
    } else {
        builder.file(record.file())
    };

    // payload
    builder = builder.payload(*record.args());

    // key-values
    #[cfg(not(feature = "serde"))]
    {
        let kvs = kv::key_values_stage_one(record.key_values());
        let new_kvs = kv::key_value_stage_two(&kvs);
        builder = builder.key_values(new_kvs.to_key_values());
        Logger::log(logger, &builder.build());
    }

    #[cfg(feature = "serde")]
    {
        let kvs = kv::key_values(record.key_values());
        builder = builder.key_values(kvs.to_key_values());
        Logger::log(logger, &builder.build());
    }
}

fn level_to_level(level: log::Level) -> logforth_core::record::Level {
    match level {
        log::Level::Error => logforth_core::record::Level::Error,
        log::Level::Warn => logforth_core::record::Level::Warn,
        log::Level::Info => logforth_core::record::Level::Info,
        log::Level::Debug => logforth_core::record::Level::Debug,
        log::Level::Trace => logforth_core::record::Level::Trace,
    }
}

#[cfg(not(feature = "serde"))]
mod kv {
    pub(super) struct KeyValuesStageOne<'a> {
        kvs: Vec<(KeyStageOne<'a>, ValueStageOne<'a>)>,
    }

    struct KeyStageOne<'a>(log::kv::Key<'a>);

    struct ValueStageOne<'a>(MaybeOwnedValue<'a>);

    enum MaybeOwnedValue<'a> {
        Borrowed(logforth_core::kv::Value<'a>),
        Owned(String),
    }

    pub(super) fn key_values_stage_one<'a>(
        source: &'a dyn log::kv::Source,
    ) -> KeyValuesStageOne<'a> {
        let mut kvs = Vec::with_capacity(log::kv::Source::count(source));

        struct KeyValueVisitor<'a, 'b> {
            kvs: &'b mut Vec<(KeyStageOne<'a>, ValueStageOne<'a>)>,
        }

        impl<'a, 'b> log::kv::VisitSource<'a> for KeyValueVisitor<'a, 'b> {
            fn visit_pair(
                &mut self,
                key: log::kv::Key<'a>,
                value: log::kv::Value<'a>,
            ) -> Result<(), log::kv::Error> {
                let key = KeyStageOne(key);
                let value = ValueStageOne(value_to_value(value));
                self.kvs.push((key, value));
                Ok(())
            }
        }

        let mut visitor = KeyValueVisitor { kvs: &mut kvs };
        log::kv::Source::visit(source, &mut visitor).unwrap();
        KeyValuesStageOne { kvs }
    }

    fn value_to_value(value: log::kv::Value) -> MaybeOwnedValue {
        struct ValueVisitor<'a>(MaybeOwnedValue<'a>);

        impl<'a> log::kv::VisitValue<'a> for ValueVisitor<'a> {
            fn visit_any(&mut self, value: log::kv::Value) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Owned(value.to_string());
                Ok(())
            }

            fn visit_null(&mut self) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::none());
                Ok(())
            }

            fn visit_u64(&mut self, value: u64) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::u64(value));
                Ok(())
            }

            fn visit_i64(&mut self, value: i64) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::i64(value));
                Ok(())
            }

            fn visit_u128(&mut self, value: u128) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::u128(value));
                Ok(())
            }

            fn visit_i128(&mut self, value: i128) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::i128(value));
                Ok(())
            }

            fn visit_f64(&mut self, value: f64) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::f64(value));
                Ok(())
            }

            fn visit_bool(&mut self, value: bool) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::bool(value));
                Ok(())
            }

            fn visit_str(&mut self, value: &str) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Owned(value.to_string());
                Ok(())
            }

            fn visit_borrowed_str(&mut self, value: &'a str) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::str(value));
                Ok(())
            }

            fn visit_char(&mut self, value: char) -> Result<(), log::kv::Error> {
                self.0 = MaybeOwnedValue::Borrowed(logforth_core::kv::Value::char(value));
                Ok(())
            }
        }

        let mut visitor = ValueVisitor(MaybeOwnedValue::Borrowed(logforth_core::kv::Value::none()));
        value.visit(&mut visitor).unwrap();
        visitor.0
    }

    pub(super) struct KeyValuesStageTwo<'a> {
        kvs: Vec<(KeyStageTwo<'a>, ValueStageTwo<'a>)>,
    }

    impl<'a> KeyValuesStageTwo<'a> {
        pub(super) fn to_key_values(&self) -> logforth_core::kv::KeyValues<'_> {
            logforth_core::kv::KeyValues::from(self.kvs.as_slice())
        }
    }

    type KeyStageTwo<'a> = logforth_core::kv::Key<'a>;

    type ValueStageTwo<'a> = logforth_core::kv::Value<'a>;

    pub(super) fn key_value_stage_two<'a>(kvs: &'a KeyValuesStageOne<'a>) -> KeyValuesStageTwo<'a> {
        let mut new_kvs = Vec::with_capacity(kvs.kvs.len());
        for (k, v) in &kvs.kvs {
            let k = logforth_core::kv::Key::borrowed(k.0.as_str());
            let v = match &v.0 {
                MaybeOwnedValue::Borrowed(v) => v.clone(),
                MaybeOwnedValue::Owned(s) => logforth_core::kv::Value::str(s.as_str()),
            };
            new_kvs.push((k, v));
        }
        KeyValuesStageTwo { kvs: new_kvs }
    }
}

#[cfg(feature = "serde")]
mod kv {
    use std::collections::HashMap;
    use std::fmt;
    use std::marker::PhantomData;

    use logforth_core::kv::KeyOwned;
    use logforth_core::kv::ValueOwned;
    use logforth_core::kv::ValueView;

    pub(super) struct KeyValues<'a> {
        kvs: Vec<(KeyOwned, ValueOwned)>,
        p: PhantomData<&'a ()>,
    }

    impl<'a> KeyValues<'a> {
        pub(super) fn to_key_values(&self) -> logforth_core::kv::KeyValues<'_> {
            logforth_core::kv::KeyValues::from(self.kvs.as_slice())
        }
    }

    pub(super) fn key_values<'a>(source: &'a dyn log::kv::Source) -> KeyValues<'a> {
        struct KeyValueVisitor(Vec<(KeyOwned, ValueOwned)>);

        impl<'a> log::kv::VisitSource<'a> for KeyValueVisitor {
            fn visit_pair(
                &mut self,
                key: log::kv::Key<'a>,
                value: log::kv::Value<'a>,
            ) -> Result<(), log::kv::Error> {
                // TODO(@tisonkun): see https://github.com/rust-lang/log/pull/727
                let key = KeyOwned::new(key.to_string());
                if let Some(value) = value_to_value(value) {
                    self.0.push((key, value));
                }
                Ok(())
            }
        }

        let mut visitor = KeyValueVisitor(Vec::with_capacity(log::kv::Source::count(source)));
        log::kv::Source::visit(source, &mut visitor).unwrap();

        KeyValues {
            kvs: visitor.0,
            p: PhantomData,
        }
    }

    // this is derived from `opentelemetry-appender-log`'s serde impl:
    // https://github.com/open-telemetry/opentelemetry-rust/blob/f7b0dd99/opentelemetry-appender-log/src/lib.rs#L304-L763
    fn value_to_value(value: impl serde::Serialize) -> Option<ValueOwned> {
        value.serialize(ValueSerializer).ok()?
    }

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
        type Ok = Option<ValueOwned>;

        type Error = ValueError;

        type SerializeSeq = ValueSerializeSeq;

        type SerializeTuple = ValueSerializeTuple;

        type SerializeTupleStruct = ValueSerializeTupleStruct;

        type SerializeTupleVariant = ValueSerializeTupleVariant;

        type SerializeMap = ValueSerializeMap;

        type SerializeStruct = ValueSerializeStruct;

        type SerializeStructVariant = ValueSerializeStructVariant;

        fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
            Ok(Some(ValueOwned::bool(v)))
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
            Ok(Some(ValueOwned::i64(v)))
        }

        fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
            if let Ok(v) = v.try_into() {
                self.serialize_i64(v)
            } else {
                self.collect_str(&v)
            }
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
            Ok(Some(ValueOwned::u64(v)))
        }

        fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
            if let Ok(v) = v.try_into() {
                self.serialize_u64(v)
            } else {
                self.collect_str(&v)
            }
        }

        fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
            self.serialize_f64(v as f64)
        }

        fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
            Ok(Some(ValueOwned::f64(v)))
        }

        fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
            Ok(Some(ValueOwned::char(v)))
        }

        fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
            Ok(Some(ValueOwned::str(v.to_string())))
        }

        fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
            Ok(Some(ValueOwned::bytes(v.to_vec())))
        }

        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            Ok(None)
        }

        fn serialize_some<T: serde::Serialize + ?Sized>(
            self,
            value: &T,
        ) -> Result<Self::Ok, Self::Error> {
            value.serialize(self)
        }

        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            Ok(None)
        }

        fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
            Ok(Some(ValueOwned::str(name)))
        }

        fn serialize_unit_variant(
            self,
            _: &'static str,
            _: u32,
            variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            Ok(Some(ValueOwned::str(variant)))
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
        type Ok = Option<ValueOwned>;

        type Error = ValueError;

        fn serialize_element<T: serde::Serialize + ?Sized>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.push(value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(ValueOwned::from_vec(self.value)))
        }
    }

    impl serde::ser::SerializeTuple for ValueSerializeTuple {
        type Ok = Option<ValueOwned>;

        type Error = ValueError;

        fn serialize_element<T: serde::Serialize + ?Sized>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.push(value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(ValueOwned::from_vec(self.value)))
        }
    }

    impl serde::ser::SerializeTupleStruct for ValueSerializeTupleStruct {
        type Ok = Option<ValueOwned>;

        type Error = ValueError;

        fn serialize_field<T: serde::Serialize + ?Sized>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.push(value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(ValueOwned::from_vec(self.value)))
        }
    }

    impl serde::ser::SerializeTupleVariant for ValueSerializeTupleVariant {
        type Ok = Option<ValueOwned>;

        type Error = ValueError;

        fn serialize_field<T: serde::Serialize + ?Sized>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.push(value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(ValueOwned::from_hash_map({
                let mut variant = HashMap::<KeyOwned, ValueOwned>::new();
                variant.insert(KeyOwned::new(self.variant), ValueOwned::list(self.value));
                variant
            })))
        }
    }

    impl serde::ser::SerializeMap for ValueSerializeMap {
        type Ok = Option<ValueOwned>;

        type Error = ValueError;

        fn serialize_key<T: serde::Serialize + ?Sized>(
            &mut self,
            key: &T,
        ) -> Result<(), Self::Error> {
            let key = match key.serialize(ValueSerializer)? {
                Some(v) => match v.view() {
                    ValueView::StaticStr(s) => KeyOwned::new(s),
                    value => KeyOwned::new(value.to_string()),
                },
                None => KeyOwned::new("None"),
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
            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.insert(key, value);
            }
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(ValueOwned::from_hash_map(self.value)))
        }
    }

    impl serde::ser::SerializeStruct for ValueSerializeStruct {
        type Ok = Option<ValueOwned>;

        type Error = ValueError;

        fn serialize_field<T: serde::Serialize + ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error> {
            let key = KeyOwned::new(key);
            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.insert(key, value);
            }
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(ValueOwned::from_hash_map(self.value)))
        }
    }

    impl serde::ser::SerializeStructVariant for ValueSerializeStructVariant {
        type Ok = Option<ValueOwned>;

        type Error = ValueError;

        fn serialize_field<T: serde::Serialize + ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error> {
            let key = KeyOwned::new(key);
            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.insert(key, value);
            }
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(ValueOwned::from_hash_map({
                let mut variant = HashMap::<KeyOwned, ValueOwned>::new();
                variant.insert(
                    KeyOwned::new(self.variant),
                    ValueOwned::from_hash_map(self.value),
                );
                variant
            })))
        }
    }
}
