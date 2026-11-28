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
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::num::NonZeroI16;
use std::num::NonZeroI128;
use std::num::NonZeroU32;
use std::num::NonZeroU128;
use std::rc::Rc;
use std::sync::Arc;

use logforth_core::Error;
use logforth_core::kv::Key;
use logforth_core::kv::KeyOwned;
use logforth_core::kv::KeyValues;
use logforth_core::kv::KeyView;
use logforth_core::kv::LogValue;
use logforth_core::kv::Value;
use logforth_core::kv::ValueOwned;
use logforth_core::kv::ValueView;
use logforth_core::record::Record;
use serde::Serialize;

#[test]
fn keys_preserve_static_provenance_and_satisfy_borrow_str_equivalence() {
    let static_key = Key::new("service.name");
    let dynamic_raw = String::from("service.name");
    let borrowed_key = Key::borrowed(&dynamic_raw);

    assert_eq!(static_key, borrowed_key);
    assert_eq!(static_key.view(), borrowed_key.view());
    assert_eq!(static_key.view().to_static_str(), Some("service.name"));
    assert_eq!(borrowed_key.view().to_static_str(), None);

    assert!(matches!(
        static_key.view().to_cow(),
        Cow::Borrowed("service.name")
    ));
    assert!(matches!(borrowed_key.view().to_cow(), Cow::Owned(_)));

    let owned_from_static = KeyOwned::from("service.name");
    let owned_from_string = KeyOwned::from(dynamic_raw.clone());
    let owned_from_cow = KeyOwned::from(Cow::Borrowed("service.name"));
    assert_eq!(owned_from_static, owned_from_string);
    assert_eq!(owned_from_static, owned_from_cow);
    assert_eq!(
        owned_from_static.view().to_static_str(),
        Some("service.name")
    );
    assert_eq!(owned_from_string.view().to_static_str(), None);

    let mut hash_map = HashMap::new();
    hash_map.insert(static_key, 1);
    assert_eq!(hash_map.get("service.name"), Some(&1));
    assert_eq!(hash_map.get(&borrowed_key), Some(&1));

    let mut btree_map = BTreeMap::new();
    btree_map.insert(owned_from_string, 2);
    assert_eq!(btree_map.get("service.name"), Some(&2));
}

#[test]
fn log_value_and_value_owned_project_scalars_containers_and_wrappers() {
    let nz_i16 = NonZeroI16::new(-12).unwrap();
    let nz_u32 = NonZeroU32::new(8080).unwrap();
    let nz_i128 = NonZeroI128::new(i128::MIN + 1).unwrap();
    let nz_u128 = NonZeroU128::new(u128::MAX).unwrap();

    assert_eq!(true.log_value().view().to_bool(), Some(true));
    assert_eq!('λ'.log_value().view().to_char(), Some('λ'));
    assert_eq!((-7_i8).log_value().view().to_i64(), Some(-7));
    assert_eq!((-8_isize).log_value().view().to_i64(), Some(-8));
    assert_eq!(42_u16.log_value().view().to_u64(), Some(42));
    assert_eq!(99_usize.log_value().view().to_u64(), Some(99));
    assert_eq!(1.5_f32.log_value().view().to_f64(), Some(1.5));
    assert_eq!(nz_i16.log_value().view().to_i64(), Some(-12));
    assert_eq!(nz_u32.log_value().view().to_u64(), Some(8080));
    assert_eq!(nz_i128.log_value().view().to_i128(), Some(i128::MIN + 1));
    assert_eq!(nz_u128.log_value().view().to_u128(), Some(u128::MAX));

    // Smart pointers, Cow, and Option delegate through `LogValue`.
    let rc_str: Rc<str> = Rc::from("rc");
    let arc_str: Arc<str> = Arc::from("arc");
    let box_bytes: Box<[u8]> = Box::from(&b"\x00\r\n\t\\\"A\x7f"[..]);
    let cow_str: Cow<'_, str> = Cow::Owned(String::from("cow"));

    assert_eq!(rc_str.log_value().view().to_str(), Some("rc"));
    assert_eq!(arc_str.log_value().view().to_str(), Some("arc"));
    assert_eq!(cow_str.log_value().view().to_str(), Some("cow"));
    assert!(None::<u64>.log_value().view().is_none());
    assert!(!Some(0_u64).log_value().view().is_none());
    assert!(!"".log_value().view().is_none());
    assert_eq!(None::<u64>.log_value().view().to_string(), "<none>");

    let bytes_view = box_bytes.log_value();
    assert_eq!(
        bytes_view.view().to_bytes(),
        Some(&b"\x00\r\n\t\\\"A\x7f"[..])
    );
    assert_eq!(
        bytes_view.view().to_string(),
        "b\"\\0\\r\\n\\t\\\\\\\"A\\x7f\""
    );

    // ValueOwned From conversions and container round-trips.
    let owned_none: ValueOwned = Option::<i32>::None.into();
    assert!(matches!(owned_none.view(), ValueView::None));

    let owned_list = ValueOwned::list([ValueOwned::from(1_i32), ValueOwned::from("two")]);
    let list_view = owned_list.log_value().view().to_list().unwrap();
    assert_eq!(list_view.len(), 2);
    assert!(!list_view.is_empty());
    let items: Vec<_> = list_view.iter().collect();
    assert_eq!(items[0].to_i64(), Some(1));
    assert_eq!(items[1].to_static_str(), Some("two"));
}

#[test]
fn value_view_integer_accessors_convert_losslessly_and_reject_out_of_range_values() {
    const {
        assert!(ValueView::None.is_none());
        assert!(matches!(ValueView::I64(8080).to_u64(), Some(8080)));
        assert!(matches!(ValueView::U64(8080).to_i64(), Some(8080)));
        assert!(matches!(ValueView::I64(8080).to_u128(), Some(8080)));
        assert!(matches!(ValueView::U128(8080).to_i128(), Some(8080)));
    }

    type Expected = (Option<i64>, Option<u64>, Option<i128>, Option<u128>);
    let i64_max_u64 = i64::MAX as u64;
    let i64_max_i128 = i128::from(i64::MAX);
    let i64_max_u128 = i64::MAX as u128;
    let i64_min_i128 = i128::from(i64::MIN);
    let u64_max_i128 = i128::from(u64::MAX);
    let u64_max_u128 = u128::from(u64::MAX);
    let i128_max_u128 = i128::MAX as u128;

    let cases: &[(ValueView<'_>, Expected)] = &[
        // `I64` boundaries
        (
            ValueView::I64(i64::MIN),
            (Some(i64::MIN), None, Some(i64_min_i128), None),
        ),
        (ValueView::I64(-1), (Some(-1), None, Some(-1), None)),
        (ValueView::I64(0), (Some(0), Some(0), Some(0), Some(0))),
        (
            ValueView::I64(i64::MAX),
            (
                Some(i64::MAX),
                Some(i64_max_u64),
                Some(i64_max_i128),
                Some(i64_max_u128),
            ),
        ),
        // `U64` boundaries
        (
            ValueView::U64(i64_max_u64),
            (
                Some(i64::MAX),
                Some(i64_max_u64),
                Some(i64_max_i128),
                Some(i64_max_u128),
            ),
        ),
        (
            ValueView::U64(i64_max_u64 + 1),
            (
                None,
                Some(i64_max_u64 + 1),
                Some(i64_max_i128 + 1),
                Some(i64_max_u128 + 1),
            ),
        ),
        (
            ValueView::U64(u64::MAX),
            (None, Some(u64::MAX), Some(u64_max_i128), Some(u64_max_u128)),
        ),
        // `I128` boundaries
        (
            ValueView::I128(i128::MIN),
            (None, None, Some(i128::MIN), None),
        ),
        (
            ValueView::I128(i64_min_i128 - 1),
            (None, None, Some(i64_min_i128 - 1), None),
        ),
        (
            ValueView::I128(i64_min_i128),
            (Some(i64::MIN), None, Some(i64_min_i128), None),
        ),
        (
            ValueView::I128(i64_max_i128),
            (
                Some(i64::MAX),
                Some(i64_max_u64),
                Some(i64_max_i128),
                Some(i64_max_u128),
            ),
        ),
        (
            ValueView::I128(i64_max_i128 + 1),
            (
                None,
                Some(i64_max_u64 + 1),
                Some(i64_max_i128 + 1),
                Some(i64_max_u128 + 1),
            ),
        ),
        (
            ValueView::I128(u64_max_i128),
            (None, Some(u64::MAX), Some(u64_max_i128), Some(u64_max_u128)),
        ),
        (
            ValueView::I128(u64_max_i128 + 1),
            (None, None, Some(u64_max_i128 + 1), Some(u64_max_u128 + 1)),
        ),
        (
            ValueView::I128(i128::MAX),
            (None, None, Some(i128::MAX), Some(i128_max_u128)),
        ),
        // `U128` boundaries
        (
            ValueView::U128(i64_max_u128),
            (
                Some(i64::MAX),
                Some(i64_max_u64),
                Some(i64_max_i128),
                Some(i64_max_u128),
            ),
        ),
        (
            ValueView::U128(i64_max_u128 + 1),
            (
                None,
                Some(i64_max_u64 + 1),
                Some(i64_max_i128 + 1),
                Some(i64_max_u128 + 1),
            ),
        ),
        (
            ValueView::U128(u64_max_u128),
            (None, Some(u64::MAX), Some(u64_max_i128), Some(u64_max_u128)),
        ),
        (
            ValueView::U128(u64_max_u128 + 1),
            (None, None, Some(u64_max_i128 + 1), Some(u64_max_u128 + 1)),
        ),
        (
            ValueView::U128(i128_max_u128),
            (None, None, Some(i128::MAX), Some(i128_max_u128)),
        ),
        (
            ValueView::U128(i128_max_u128 + 1),
            (None, None, None, Some(i128_max_u128 + 1)),
        ),
        (
            ValueView::U128(u128::MAX),
            (None, None, None, Some(u128::MAX)),
        ),
        // Non-integer variants are never coerced.
        (ValueView::F64(42.0), (None, None, None, None)),
        (ValueView::StaticStr("42"), (None, None, None, None)),
        (ValueView::Bool(true), (None, None, None, None)),
        (ValueView::None, (None, None, None, None)),
    ];

    for &(view, expected) in cases {
        let actual = (view.to_i64(), view.to_u64(), view.to_i128(), view.to_u128());
        assert_eq!(actual, expected, "mismatch for view {view}");
    }
    // `to_f64` remains strictly `ValueView::F64` without lossy integer coercion.
    assert_eq!(ValueView::I64(42).to_f64(), None);
    assert_eq!(ValueView::U64(42).to_f64(), None);
}

#[test]
fn container_display_formats_values_consistently_across_sync_and_owned_paths() {
    let dynamic_str = String::from("borrowed");
    let dynamic_key = String::from("dyn_key");
    let inner_map = [
        (Key::new("ok"), Value::bool(true)),
        (Key::borrowed(&dynamic_key), Value::u64(7)),
    ];
    let list_items = [
        Value::i64(-1),
        Value::str(&dynamic_str),
        Value::static_str("static"),
        Value::char('x'),
        Value::none(),
        Value::bytes(b"\x00\nA"),
        Value::map(&inner_map),
    ];
    let top_kvs = [
        (Key::new("items"), Value::list(&list_items)),
        (Key::new("meta"), Value::map(&inner_map)),
    ];

    let expected_list =
        "[-1, \"borrowed\", \"static\", 'x', <none>, b\"\\0\\nA\", {\"ok\": true, \"dyn_key\": 7}]";
    let expected_map = "{\"ok\": true, \"dyn_key\": 7}";

    let record = Record::builder().key_values(top_kvs.as_slice()).build();
    let verify = |r: &Record<'_>| {
        let mut iter = r.key_values().iter();
        let (_, first_val) = iter.next().unwrap();
        assert_eq!(first_val.to_string(), expected_list);

        let (_, second_val) = iter.clone().next().unwrap();
        assert_eq!(second_val.to_string(), expected_map);
    };

    verify(&record);
    record.to_owned().with(|owned_record| verify(&owned_record));
}

#[test]
fn duplicate_keys_preserve_insertion_order_and_resolve_get_with_last_entry_wins() {
    let k = Key::new("dup");
    let other = Key::new("other");
    let inner_entries = [
        (k, Value::i64(1)),
        (other, Value::static_str("keep")),
        (k, Value::i64(2)),
    ];
    let outer_entries = [
        (k, Value::static_str("shadowed")),
        (other, Value::u64(10)),
        (k, Value::map(&inner_entries)),
    ];

    let record = Record::builder()
        .key_values(outer_entries.as_slice())
        .build();

    let verify = |r: &Record<'_>| {
        let kvs = r.key_values();
        assert_eq!(kvs.len(), 3);
        assert!(!kvs.is_empty());

        let visited_keys: Vec<_> = kvs.iter().map(|(key, _)| key.as_str().to_owned()).collect();
        assert_eq!(visited_keys, ["dup", "other", "dup"]);

        let inner_map = kvs.get("dup").unwrap().to_map().unwrap();
        assert_eq!(inner_map.len(), 3);
        assert_eq!(inner_map.get("dup").unwrap().to_i64(), Some(2));
        assert_eq!(inner_map.get("other").unwrap().to_str(), Some("keep"));
        assert!(inner_map.get("missing").is_none());
        assert!(kvs.get("missing").is_none());
    };

    verify(&record);
    record.to_owned().with(|owned_record| verify(&owned_record));

    // Visitor errors stop iteration immediately.
    let mut visited = 0usize;
    let err = KeyValues::from(outer_entries.as_slice())
        .visit(&mut |_: KeyView, _: ValueView| {
            visited += 1;
            Err(Error::new("stop early"))
        })
        .unwrap_err();
    assert_eq!(visited, 1);
    assert_eq!(err.message(), "stop early");
}

#[derive(Serialize)]
struct UnitMarker;

#[derive(Serialize)]
struct NewtypeId(u64);

#[derive(Serialize)]
enum EventShape {
    Unit,
    Newtype(u32),
    Tuple(&'static str, bool),
    Struct { code: i32, retry: bool },
}

#[derive(Serialize)]
struct ComplexPayload {
    unit: (),
    marker: UnitMarker,
    id: NewtypeId,
    big_signed: i128,
    big_unsigned: u128,
    bytes: StatusBytes,
    variants: Vec<EventShape>,
    numeric_map: BTreeMap<u32, &'static str>,
}

struct StatusBytes;

impl Serialize for StatusBytes {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(b"\xde\xad")
    }
}

struct FallibleSerialize;

impl Serialize for FallibleSerialize {
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("cannot serialize secret"))
    }
}

#[test]
fn serde_captures_structured_trees_and_degrades_gracefully_on_failure() {
    let payload = ComplexPayload {
        unit: (),
        marker: UnitMarker,
        id: NewtypeId(42),
        big_signed: i128::MIN,
        big_unsigned: u128::MAX,
        bytes: StatusBytes,
        variants: vec![
            EventShape::Unit,
            EventShape::Newtype(7),
            EventShape::Tuple("ok", true),
            EventShape::Struct {
                code: 500,
                retry: false,
            },
        ],
        numeric_map: BTreeMap::from([(200, "ok")]),
    };

    let owned = ValueOwned::try_from_serde(&payload).unwrap();
    let root = owned.view().to_map().unwrap();

    assert!(matches!(root.get("unit").unwrap(), ValueView::None));
    assert_eq!(
        root.get("marker").unwrap().to_static_str(),
        Some("UnitMarker")
    );
    assert_eq!(root.get("id").unwrap().to_u64(), Some(42));
    assert_eq!(root.get("big_signed").unwrap().to_i128(), Some(i128::MIN));
    assert_eq!(root.get("big_unsigned").unwrap().to_u128(), Some(u128::MAX));
    assert_eq!(
        root.get("bytes").unwrap().to_bytes(),
        Some(&b"\xde\xad"[..])
    );

    let variants: Vec<_> = root
        .get("variants")
        .unwrap()
        .to_list()
        .unwrap()
        .iter()
        .collect();
    assert_eq!(variants[0].to_static_str(), Some("Unit"));
    assert_eq!(
        variants[1]
            .to_map()
            .unwrap()
            .get("Newtype")
            .unwrap()
            .to_u64(),
        Some(7)
    );
    let tuple_items: Vec<_> = variants[2]
        .to_map()
        .unwrap()
        .get("Tuple")
        .unwrap()
        .to_list()
        .unwrap()
        .iter()
        .collect();
    assert_eq!(tuple_items[0].to_str(), Some("ok"));
    assert_eq!(tuple_items[1].to_bool(), Some(true));
    let struct_variant = variants[3]
        .to_map()
        .unwrap()
        .get("Struct")
        .unwrap()
        .to_map()
        .unwrap();
    assert_eq!(struct_variant.get("code").unwrap().to_i64(), Some(500));
    assert_eq!(struct_variant.get("retry").unwrap().to_bool(), Some(false));

    let numeric_map = root.get("numeric_map").unwrap().to_map().unwrap();
    assert_eq!(numeric_map.get("200").unwrap().to_str(), Some("ok"));

    // Round-trip `ValueOwned` back through `serde::Serialize` via `ValueOwned::try_from_serde`.
    let reserialized = ValueOwned::try_from_serde(&owned).unwrap();
    assert_eq!(
        reserialized
            .view()
            .to_map()
            .unwrap()
            .get("big_unsigned")
            .unwrap()
            .to_u128(),
        Some(u128::MAX)
    );

    // Fallible `Serialize` implementations return `Err` or a non-panicking placeholder string.
    let err = ValueOwned::try_from_serde(&FallibleSerialize).unwrap_err();
    assert_eq!(err.message(), "cannot serialize secret");
    let fallback = ValueOwned::from_serde(&FallibleSerialize);
    assert_eq!(
        fallback.view().to_str(),
        Some("<serde error: cannot serialize secret>")
    );
}
