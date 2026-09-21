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

#![cfg(feature = "serde")]

use logforth_core::kv::ValueOwned;

#[test]
fn nested_variants_bytes_and_wide_numbers_retain_their_structure() {
    #[derive(serde::Serialize)]
    enum State {
        Idle,
        Count(u128),
        Pair(i128, bool),
        Active { name: String },
    }
    #[derive(serde::Serialize)]
    struct Snapshot {
        states: Vec<State>,
        missing: Option<bool>,
    }
    let input = Snapshot {
        states: vec![
            State::Idle,
            State::Count(u128::MAX),
            State::Pair(i128::MIN, true),
            State::Active {
                name: "worker".into(),
            },
        ],
        missing: None,
    };
    let owned = ValueOwned::from_serde(&input).unwrap();
    let map = owned.view().to_map().unwrap();
    let states: Vec<_> = map
        .get("states")
        .unwrap()
        .to_list()
        .unwrap()
        .iter()
        .collect();
    assert_eq!(states[0].to_str(), Some("Idle"));
    assert_eq!(
        states[1].to_map().unwrap().get("Count").unwrap().to_u128(),
        Some(u128::MAX)
    );
    let pair: Vec<_> = states[2]
        .to_map()
        .unwrap()
        .get("Pair")
        .unwrap()
        .to_list()
        .unwrap()
        .iter()
        .collect();
    assert_eq!(pair[0].to_i128(), Some(i128::MIN));
    assert_eq!(pair[1].to_bool(), Some(true));
    assert_eq!(
        states[3]
            .to_map()
            .unwrap()
            .get("Active")
            .unwrap()
            .to_map()
            .unwrap()
            .get("name")
            .unwrap()
            .to_str(),
        Some("worker")
    );
    assert!(matches!(
        map.get("missing"),
        Some(logforth_core::kv::ValueView::None)
    ));

    struct Bytes;
    impl serde::Serialize for Bytes {
        fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_bytes(&[0, 255])
        }
    }
    assert!(matches!(
        ValueOwned::from_serde(&Bytes).unwrap().view(),
        logforth_core::kv::ValueView::Bytes(&[0, 255])
    ));
}

#[test]
fn serialization_errors_are_reported_and_disabled_events_do_not_serialize() {
    struct Broken;
    impl serde::Serialize for Broken {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("snapshot unavailable"))
        }
    }
    let err = ValueOwned::from_serde(&Broken).unwrap_err();
    assert!(err.to_string().contains("snapshot unavailable"));
    let captured = logforth_core::kv::serde(&Broken);
    assert!(
        captured
            .view()
            .to_str()
            .unwrap()
            .starts_with("<serialization failed:")
    );
    assert!(
        captured
            .view()
            .to_str()
            .unwrap()
            .contains("snapshot unavailable")
    );
    let logger = logforth_core::builder().build();
    logforth_core::info!(logger, {
        "snapshot" => ValueOwned::from_serde(&Broken).expect("must remain unevaluated"),
    });
}
