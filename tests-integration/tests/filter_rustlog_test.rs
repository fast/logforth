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

use std::error::Error as _;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use logforth::Diagnostic;
use logforth::Filter;
use logforth::LoggerBuilder;
use logforth::filter::FilterResult;
use logforth::filter::RustLogFilter;
use logforth::filter::rustlog::DEFAULT_FILTER_ENV;
use logforth::filter::rustlog::RustLogFilterBuilder;
use logforth::kv::Key;
use logforth::kv::Value;
use logforth::record::Level;
use logforth::record::LevelFilter;
use logforth::record::Record;
use tests_integration::RecordingAppender;

const EMPTY_BUILDER: RustLogFilterBuilder = RustLogFilterBuilder::new();

fn accepts(filter: &RustLogFilter, level: Level, module: Option<&str>) -> bool {
    let record = Record::builder().level(level).module_path(module).build();
    filter.filter(&record, &()) != FilterResult::Reject
}

#[test]
fn empty_and_whitespace_builders_default_to_error_while_prefix_only_silences_unmatched() {
    for filter in [
        EMPTY_BUILDER.clone().build(),
        RustLogFilterBuilder::default().build(),
        RustLogFilterBuilder::from_spec("").unwrap().build(),
        RustLogFilterBuilder::from_spec("  ,  , ").unwrap().build(),
        "".parse::<RustLogFilter>().unwrap(),
        RustLogFilter::try_from("   ").unwrap(),
    ] {
        for module in [None, Some(""), Some("any_crate::sub")] {
            assert!(!accepts(&filter, Level::Warn4, module));
            assert!(accepts(&filter, Level::Error, module));
            assert!(accepts(&filter, Level::Fatal4, module));
        }
    }

    // Prefix-only specification leaves unmatched modules silent (`Off`), even at `Fatal4`.
    let prefix_only: RustLogFilter = "my_app=debug".parse().unwrap();
    assert!(accepts(&prefix_only, Level::Debug, Some("my_app::worker")));
    assert!(!accepts(
        &prefix_only,
        Level::Trace4,
        Some("my_app::worker")
    ));
    assert!(!accepts(&prefix_only, Level::Fatal4, Some("other_crate")));
    assert!(!accepts(&prefix_only, Level::Fatal4, None));

    // `From<LevelFilter>` preserves arbitrary `LevelFilter` predicates (such as `Equal`).
    let exact_info = RustLogFilter::from(LevelFilter::Equal(Level::Info));
    assert!(!accepts(&exact_info, Level::Debug, Some("app")));
    assert!(accepts(&exact_info, Level::Info, Some("app")));
    assert!(!accepts(&exact_info, Level::Error, Some("app")));
}

#[test]
fn directive_syntax_supports_all_twenty_four_levels_pseudo_levels_and_last_write_wins() {
    for (idx, level) in Level::ALL.into_iter().enumerate() {
        let mixed_case: String = level
            .name()
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i % 2 == 0 {
                    c.to_ascii_lowercase()
                } else {
                    c
                }
            })
            .collect();

        let filter: RustLogFilter = mixed_case.parse().unwrap();
        for (cand_idx, candidate) in Level::ALL.into_iter().enumerate() {
            assert_eq!(
                accepts(&filter, candidate, Some("app")),
                cand_idx >= idx,
                "failed for spec {mixed_case} on candidate {candidate}"
            );
        }
    }

    let all_filter: RustLogFilter = "aLl".parse().unwrap();
    let off_filter: RustLogFilter = "oFf".parse().unwrap();
    for level in Level::ALL {
        assert!(accepts(&all_filter, level, Some("app")));
        assert!(!accepts(&off_filter, level, Some("app")));
    }

    // Bare `prefix` and `prefix=` enable `All`, `=level` and `=` set fallback, whitespace is
    // trimmed, and later duplicates win.
    let builder = RustLogFilterBuilder::from_spec(
        " info , = warn , bare_mod , eq_empty = , dup = debug , dup = error2 ",
    )
    .unwrap();
    let layered = builder.build();

    assert!(!accepts(&layered, Level::Info4, Some("unmatched")));
    assert!(accepts(&layered, Level::Warn, Some("unmatched")));
    assert!(accepts(&layered, Level::Trace, Some("bare_mod::child")));
    assert!(accepts(&layered, Level::Trace, Some("eq_empty::child")));
    assert!(!accepts(&layered, Level::Error, Some("dup::child")));
    assert!(accepts(&layered, Level::Error2, Some("dup::child")));

    let eq_all = RustLogFilterBuilder::from_spec(" = , quiet = error ")
        .unwrap()
        .build();
    assert!(accepts(&eq_all, Level::Trace, Some("unmatched")));
    assert!(!accepts(&eq_all, Level::Warn, Some("quiet::sub")));
    assert!(accepts(&eq_all, Level::Error, Some("quiet::sub")));

    // Empty prefix in `Builder::prefix` normalizes to `fallback`, so later calls in either order
    // override cleanly without leaving a shadowing `""` prefix entry.
    let prefix_then_fallback = RustLogFilterBuilder::new()
        .prefix("", LevelFilter::All)
        .fallback(LevelFilter::MoreSevereEqual(Level::Warn))
        .build();
    assert!(!accepts(&prefix_then_fallback, Level::Info, Some("app")));
    assert!(accepts(&prefix_then_fallback, Level::Warn, Some("app")));

    let fallback_then_prefix = RustLogFilterBuilder::new()
        .fallback(LevelFilter::MoreSevereEqual(Level::Error))
        .prefix("", LevelFilter::MoreSevereEqual(Level::Debug))
        .build();
    assert!(accepts(&fallback_then_prefix, Level::Debug, Some("app")));
    assert!(!accepts(&fallback_then_prefix, Level::Trace, Some("app")));

    // Post-editing a parsed builder overrides specific prefixes and fallback cleanly.
    let post_edited_builder = RustLogFilterBuilder::from_spec("debug,hyper=trace").unwrap();
    let post_edited = post_edited_builder
        .prefix("hyper", LevelFilter::MoreSevereEqual(Level::Warn))
        .build();
    assert!(accepts(&post_edited, Level::Debug, Some("my_app")));
    assert!(!accepts(&post_edited, Level::Info, Some("hyper::client")));
    assert!(accepts(&post_edited, Level::Warn, Some("hyper::client")));
}

#[test]
fn longest_prefix_wins_and_subtree_overrides_narrow_or_widen_parent_rules() {
    let sink = RecordingAppender::default();
    let builder = RustLogFilterBuilder::from_spec(
        "error,app=info,app::db=warn,app::db::pool=trace,app::noisy=off",
    )
    .unwrap();
    let filter = builder.build();
    let logger = LoggerBuilder::new()
        .dispatch(|d| d.filter(filter.clone()).append(sink.clone()))
        .build();

    let cases = [
        ("other", Level::Warn, false),
        ("other", Level::Error, true),
        ("app", Level::Debug, false),
        ("app", Level::Info, true),
        ("app::db", Level::Info, false),
        ("app::db", Level::Warn, true),
        ("app::db::pool::conn", Level::Trace, true),
        ("app::noisy::inner", Level::Fatal4, false),
    ];

    for (module, level, should_pass) in cases {
        let msg = format!("{module}:{level}");
        logger.log(
            &Record::builder()
                .level(level)
                .module_path(Some(module))
                .payload(format_args!("{msg}"))
                .build(),
        );
        assert_eq!(
            accepts(&filter, level, Some(module)),
            should_pass,
            "unexpected result for {msg}"
        );
    }

    assert_eq!(
        sink.payloads(),
        [
            "other:ERROR",
            "app:INFO",
            "app::db:WARN",
            "app::db::pool::conn:TRACE",
        ]
    );
}

#[derive(Debug)]
struct CountingDownstreamFilter {
    calls: AtomicUsize,
}

impl Filter for CountingDownstreamFilter {
    fn filter_level(&self, _: Level) -> FilterResult {
        FilterResult::Neutral
    }

    fn filter(&self, record: &Record, _: &dyn Diagnostic) -> FilterResult {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if record.payload().to_string() == "reject-downstream" {
            FilterResult::Reject
        } else {
            FilterResult::Neutral
        }
    }
}

#[test]
fn dual_level_bitmasks_prune_disabled_levels_and_preserve_downstream_filters() {
    let downstream = Arc::new(CountingDownstreamFilter {
        calls: AtomicUsize::new(0),
    });
    let sink = RecordingAppender::default();

    // Fallback is `warn`, `app::sub` is `info`:
    // - `Trace..=Debug4`: rejected by all rules (`may_neutral == 0`) -> pruned at
    //   `Logger::enabled`.
    // - `Warn..=Fatal4`: accepted by fallback AND all prefixes (`always_neutral == 1`), yet must
    //   still return `FilterResult::Neutral` so `downstream` executes!
    let builder = RustLogFilterBuilder::from_spec("warn,app::sub=info").unwrap();
    let filter = builder.build();
    let logger = LoggerBuilder::new()
        .dispatch(|d| {
            d.filter(filter)
                .filter(downstream.clone())
                .append(sink.clone())
        })
        .build();

    assert!(!logger.enabled(Level::Debug4));
    assert!(logger.enabled(Level::Info));
    assert!(logger.enabled(Level::Error));

    logger.log(
        &Record::builder()
            .level(Level::Debug)
            .module_path_static("app::sub")
            .payload(format_args!("pruned"))
            .build(),
    );
    assert_eq!(downstream.calls.load(Ordering::Relaxed), 0);

    // `Error` hits `always_neutral` fast path in `RustLogFilter`, then `downstream` rejects one
    // and lets the other pass.
    logger.log(
        &Record::builder()
            .level(Level::Error)
            .module_path_static("unmatched")
            .payload(format_args!("reject-downstream"))
            .build(),
    );
    logger.log(
        &Record::builder()
            .level(Level::Error)
            .module_path_static("unmatched")
            .payload(format_args!("keep-downstream"))
            .build(),
    );

    assert_eq!(downstream.calls.load(Ordering::Relaxed), 2);
    assert_eq!(sink.payloads(), ["keep-downstream"]);
}

#[test]
fn logforth_target_kv_overrides_module_path_without_colliding_with_user_target_field() {
    let filter = RustLogFilterBuilder::from_spec("info,custom::audit=trace,app::quiet=error")
        .unwrap()
        .build();

    // Explicit `"logforth.target"` overrides `module_path` (even when `module_path` is quieter or
    // `None`).
    let audit_kvs = [(
        Key::new("logforth.target"),
        Value::static_str("custom::audit"),
    )];
    let audit_record = Record::builder()
        .level(Level::Trace)
        .module_path_static("app::quiet")
        .key_values(audit_kvs.as_slice())
        .build();
    assert_eq!(filter.filter(&audit_record, &()), FilterResult::Neutral);

    let audit_no_mod = Record::builder()
        .level(Level::Debug)
        .key_values(audit_kvs.as_slice())
        .build();
    assert_eq!(filter.filter(&audit_no_mod, &()), FilterResult::Neutral);

    // Explicit empty `"logforth.target"` still overrides `module_path` and routes to `fallback`
    // rather than falling back to `module_path`.
    let empty_target_kvs = [(Key::new("logforth.target"), Value::static_str(""))];
    let empty_target_record = Record::builder()
        .level(Level::Info)
        .module_path_static("app::quiet")
        .key_values(empty_target_kvs.as_slice())
        .build();
    assert_eq!(
        filter.filter(&empty_target_record, &()),
        FilterResult::Neutral
    );

    // Non-string `"logforth.target"` falls back to `Record::module_path`.
    let numeric_target_kvs = [(Key::new("logforth.target"), Value::u64(42))];
    let fallback_to_mod = Record::builder()
        .level(Level::Warn)
        .module_path_static("app::quiet")
        .key_values(numeric_target_kvs.as_slice())
        .build();
    assert_eq!(filter.filter(&fallback_to_mod, &()), FilterResult::Reject);

    // Application `"target"` key-value never collides with reserved `"logforth.target"`.
    let app_target_kvs = [(Key::new("target"), Value::static_str("app::quiet"))];
    let app_record = Record::builder()
        .level(Level::Info)
        .module_path_static("app::service")
        .key_values(app_target_kvs.as_slice())
        .build();
    assert_eq!(filter.filter(&app_record, &()), FilterResult::Neutral);
}

#[test]
fn malformed_directives_fail_fast_on_first_error() {
    for bad_spec in ["=not_a_level", "==", "a=b=c", "a==", "crate1=not_a_level"] {
        assert!(
            RustLogFilterBuilder::from_spec(bad_spec).is_err(),
            "expected error for malformed spec {bad_spec:?}"
        );
    }

    let err = RustLogFilterBuilder::from_spec("ok=info,first=bad_one,second=bad=two").unwrap_err();
    assert_eq!(err.message(), "malformed logging spec 'bad_one'");
}

const CHILD_MODE_ENV: &str = "LOGFORTH_TEST_RUSTLOG_CHILD_MODE";
const CUSTOM_LOG_ENV: &str = "LOGFORTH_TEST_CUSTOM_LOG";

#[test]
fn from_env_and_from_default_env_handle_unset_set_and_non_utf8_in_isolated_subprocesses() {
    if let Ok(mode) = std::env::var(CHILD_MODE_ENV) {
        match mode.as_str() {
            "unset" => {
                let default_env = RustLogFilterBuilder::from_default_env().unwrap().build();
                assert!(!accepts(&default_env, Level::Warn, Some("app")));
                assert!(accepts(&default_env, Level::Error, Some("app")));

                let fallback_env = RustLogFilterBuilder::from_default_env_or("debug")
                    .unwrap()
                    .build();
                assert!(accepts(&fallback_env, Level::Debug, Some("app")));
                assert!(!accepts(&fallback_env, Level::Trace, Some("app")));
            }
            "set" => {
                let from_default = RustLogFilterBuilder::from_default_env_or("error")
                    .unwrap()
                    .build();
                assert!(accepts(&from_default, Level::Trace, Some("my_srv::api")));
                assert!(!accepts(&from_default, Level::Info, Some("other")));

                let from_custom = RustLogFilterBuilder::from_env_or(CUSTOM_LOG_ENV, "error")
                    .unwrap()
                    .build();
                assert!(accepts(&from_custom, Level::Info2, Some("custom_mod")));
                assert!(!accepts(&from_custom, Level::Info, Some("custom_mod")));
            }
            "non_utf8" => {
                let err = RustLogFilterBuilder::from_env(CUSTOM_LOG_ENV).unwrap_err();
                assert!(err.message().contains(CUSTOM_LOG_ENV));
                assert!(err.source().unwrap().is::<std::env::VarError>());
            }
            other => panic!("unknown child mode: {other}"),
        }
        return;
    }

    let exe = std::env::current_exe().unwrap();
    let test_name =
        "from_env_and_from_default_env_handle_unset_set_and_non_utf8_in_isolated_subprocesses";

    let status_unset = Command::new(&exe)
        .args(["--exact", test_name, "--nocapture"])
        .env(CHILD_MODE_ENV, "unset")
        .env_remove(DEFAULT_FILTER_ENV)
        .env_remove(CUSTOM_LOG_ENV)
        .status()
        .unwrap();
    assert!(status_unset.success());

    let status_set = Command::new(&exe)
        .args(["--exact", test_name, "--nocapture"])
        .env(CHILD_MODE_ENV, "set")
        .env(DEFAULT_FILTER_ENV, "my_srv=trace")
        .env(CUSTOM_LOG_ENV, "custom_mod=info2")
        .status()
        .unwrap();
    assert!(status_set.success());

    #[cfg(unix)]
    {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt as _;

        let status_non_utf8 = Command::new(&exe)
            .args(["--exact", test_name, "--nocapture"])
            .env(CHILD_MODE_ENV, "non_utf8")
            .env(CUSTOM_LOG_ENV, OsStr::from_bytes(b"\xff\xfe"))
            .status()
            .unwrap();
        assert!(status_non_utf8.success());
    }
}
