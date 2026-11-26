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

use insta::assert_snapshot;
use logforth_core::Filter;
use logforth_core::filter::FilterResult;
use logforth_core::kv::Key;
use logforth_core::kv::KeyValues;
use logforth_core::kv::Value;
use logforth_core::record::Level;
use logforth_core::record::LevelFilter;
use logforth_core::record::Record;

use crate::Directive;
use crate::RustLogFilter;
use crate::RustLogFilterBuilder;
use crate::parse_spec;

impl RustLogFilter {
    fn rejected(&self, level: Level, name: &str) -> bool {
        let record = Record::builder()
            .level(level)
            .module_path(Some(name))
            .build();
        matches!(Filter::filter(self, &record, &()), FilterResult::Reject)
    }
}

#[test]
fn parse_spec_valid() {
    let dirs = parse_spec("crate1::mod1=error,crate1::mod2,crate2=debug").unwrap();

    assert_eq!(dirs.len(), 3);
    assert_eq!(dirs[0].prefix, Some("crate1::mod1".to_owned()));
    assert_eq!(dirs[0].level, LevelFilter::MoreSevereEqual(Level::Error));

    assert_eq!(dirs[1].prefix, Some("crate1::mod2".to_owned()));
    assert_eq!(dirs[1].level, LevelFilter::All);

    assert_eq!(dirs[2].prefix, Some("crate2".to_owned()));
    assert_eq!(dirs[2].level, LevelFilter::MoreSevereEqual(Level::Debug));
}

#[test]
fn parse_spec_invalid_crate() {
    // test parse_spec with multiple = in specification
    let err = parse_spec("crate1::mod1=warn=info,crate2=debug").unwrap_err();
    assert_snapshot!(
        err.to_string(),
        @"malformed logging spec 'crate1::mod1=warn=info'"
    );
}

#[test]
fn parse_spec_empty_target_before_equals() {
    let err = parse_spec("=debug").unwrap_err();
    assert_snapshot!(err.to_string(), @"malformed logging spec '=debug'");
}

#[test]
fn parse_spec_invalid_level() {
    // test parse_spec with 'noNumber' as log level
    let err = parse_spec("crate1::mod1=noNumber,crate2=debug").unwrap_err();
    assert_snapshot!(err.to_string(), @"malformed logging spec 'noNumber'");
}

#[test]
fn parse_spec_string_level() {
    // test parse_spec with 'wrong' as log level
    let err = parse_spec("crate1::mod1=wrong,crate2=warn").unwrap_err();
    assert_snapshot!(err.to_string(), @"malformed logging spec 'wrong'");
}

#[test]
fn parse_spec_empty_level() {
    // test parse_spec with '' as log level (valid) vs invalid level
    let dirs = parse_spec("crate2=").unwrap();
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].prefix, Some("crate2".to_owned()));
    assert_eq!(dirs[0].level, LevelFilter::All);

    let err = parse_spec("crate1::mod1=wrong,crate2=").unwrap_err();
    assert_snapshot!(err.to_string(), @"malformed logging spec 'wrong'");
}

#[test]
fn parse_spec_empty_level_isolated() {
    // test parse_spec with "" as log level (and the entire spec str)
    let dirs = parse_spec("").unwrap(); // should be ignored
    assert_eq!(dirs.len(), 0);
}

#[test]
fn parse_spec_blank_level_isolated() {
    // test parse_spec with a white-space-only string specified as the log
    // level (and the entire spec str)
    let dirs = parse_spec("     ").unwrap(); // should be ignored
    assert_eq!(dirs.len(), 0);
}

#[test]
fn parse_spec_blank_level_isolated_comma_only() {
    // The spec should contain zero or more comma-separated string slices,
    // so a comma-only string should be interpreted as two empty strings
    // (which should both be ignored).
    let dirs = parse_spec(",").unwrap();
    assert_eq!(dirs.len(), 0);
}

#[test]
fn parse_spec_blank_level_isolated_comma_blank() {
    let dirs = parse_spec(",     ").unwrap();
    assert_eq!(dirs.len(), 0);
}

#[test]
fn parse_spec_blank_level_isolated_blank_comma() {
    let dirs = parse_spec("     ,").unwrap();
    assert_eq!(dirs.len(), 0);
}

#[test]
fn parse_spec_global() {
    // test parse_spec with no crate
    let dirs = parse_spec("warn,crate2=debug").unwrap();
    assert_eq!(dirs.len(), 2);
    assert_eq!(dirs[0].prefix, None);
    assert_eq!(dirs[0].level, LevelFilter::MoreSevereEqual(Level::Warn));
    assert_eq!(dirs[1].prefix, Some("crate2".to_owned()));
    assert_eq!(dirs[1].level, LevelFilter::MoreSevereEqual(Level::Debug));
}

#[test]
fn parse_spec_global_bare_warn_lc() {
    // test parse_spec with no crate, in isolation, all lowercase
    let dirs = parse_spec("warn").unwrap();
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].prefix, None);
    assert_eq!(dirs[0].level, LevelFilter::MoreSevereEqual(Level::Warn));
}

#[test]
fn parse_spec_global_bare_warn_uc() {
    // test parse_spec with no crate, in isolation, all uppercase
    let dirs = parse_spec("WARN").unwrap();
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].prefix, None);
    assert_eq!(dirs[0].level, LevelFilter::MoreSevereEqual(Level::Warn));
}

#[test]
fn parse_spec_global_bare_warn_mixed() {
    // test parse_spec with no crate, in isolation, mixed case
    let dirs = parse_spec("wArN").unwrap();
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].prefix, None);
    assert_eq!(dirs[0].level, LevelFilter::MoreSevereEqual(Level::Warn));
}

#[test]
fn parse_spec_fails_fast_on_first_error() {
    let err = parse_spec("crate2=debug,crate1::mod1=warn=info,crate3=invalid").unwrap_err();
    assert_snapshot!(
        err.to_string(),
        @"malformed logging spec 'crate1::mod1=warn=info'"
    );

    let err = parse_spec("crate2=debug,crate1::mod1=noNumber,crate3=invalid").unwrap_err();
    assert_snapshot!(err.to_string(), @"malformed logging spec 'noNumber'");
}

#[test]
fn filter_info() {
    let logger = RustLogFilterBuilder::default()
        .fallback(LevelFilter::MoreSevereEqual(Level::Info))
        .build();
    assert!(!logger.rejected(Level::Info, "crate1"));
    assert!(logger.rejected(Level::Debug, "crate1"));
}

#[test]
fn filter_beginning_longest_match() {
    let logger = RustLogFilterBuilder::default()
        .prefix("crate2", LevelFilter::MoreSevereEqual(Level::Info))
        .prefix("crate2::mod", LevelFilter::MoreSevereEqual(Level::Debug))
        .prefix("crate1::mod1", LevelFilter::MoreSevereEqual(Level::Warn))
        .build();
    assert!(!logger.rejected(Level::Debug, "crate2::mod1"));
    assert!(logger.rejected(Level::Debug, "crate2"));
}

// Some of our tests are only correct or complete when they cover the full
// universe of variants for Level. In the unlikely event that a new
// variant is added in the future, this test will detect the scenario and
// alert us to the need to review and update the tests.
#[test]
fn ensure_tests_cover_level_universe() {
    let level_universe: Level = Level::Trace; // use of trace variant is arbitrary
    match level_universe {
        Level::Trace
        | Level::Trace2
        | Level::Trace3
        | Level::Trace4
        | Level::Debug
        | Level::Debug2
        | Level::Debug3
        | Level::Debug4
        | Level::Info
        | Level::Info2
        | Level::Info3
        | Level::Info4
        | Level::Warn
        | Level::Warn2
        | Level::Warn3
        | Level::Warn4
        | Level::Error
        | Level::Error2
        | Level::Error3
        | Level::Error4
        | Level::Fatal
        | Level::Fatal2
        | Level::Fatal3
        | Level::Fatal4 => (),
    }
}

#[test]
fn parse_default() {
    let builder = RustLogFilterBuilder::from_spec("info,crate1::mod1=warn").unwrap();
    let logger = builder.build();
    assert!(!logger.rejected(Level::Warn, "crate1::mod1"));
    assert!(logger.rejected(Level::Info, "crate1::mod1"));
    assert!(!logger.rejected(Level::Info, "crate2::mod2"));
}

#[test]
fn parse_default_bare_level_off_lc() {
    let builder = RustLogFilterBuilder::from_spec("off").unwrap();
    let logger = builder.build();
    assert!(logger.rejected(Level::Error, ""));
    assert!(logger.rejected(Level::Warn, ""));
    assert!(logger.rejected(Level::Info, ""));
    assert!(logger.rejected(Level::Debug, ""));
    assert!(logger.rejected(Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_off_uc() {
    let builder = RustLogFilterBuilder::from_spec("OFF").unwrap();
    let logger = builder.build();
    assert!(logger.rejected(Level::Error, ""));
    assert!(logger.rejected(Level::Warn, ""));
    assert!(logger.rejected(Level::Info, ""));
    assert!(logger.rejected(Level::Debug, ""));
    assert!(logger.rejected(Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_error_lc() {
    let builder = RustLogFilterBuilder::from_spec("error").unwrap();
    let logger = builder.build();
    assert!(!logger.rejected(Level::Error, ""));
    assert!(logger.rejected(Level::Warn, ""));
    assert!(logger.rejected(Level::Info, ""));
    assert!(logger.rejected(Level::Debug, ""));
    assert!(logger.rejected(Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_error_uc() {
    let builder = RustLogFilterBuilder::from_spec("ERROR").unwrap();
    let logger = builder.build();
    assert!(!logger.rejected(Level::Error, ""));
    assert!(logger.rejected(Level::Warn, ""));
    assert!(logger.rejected(Level::Info, ""));
    assert!(logger.rejected(Level::Debug, ""));
    assert!(logger.rejected(Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_warn_lc() {
    let builder = RustLogFilterBuilder::from_spec("warn").unwrap();
    let logger = builder.build();
    assert!(!logger.rejected(Level::Error, ""));
    assert!(!logger.rejected(Level::Warn, ""));
    assert!(logger.rejected(Level::Info, ""));
    assert!(logger.rejected(Level::Debug, ""));
    assert!(logger.rejected(Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_warn_uc() {
    let builder = RustLogFilterBuilder::from_spec("WARN").unwrap();
    let logger = builder.build();
    assert!(!logger.rejected(Level::Error, ""));
    assert!(!logger.rejected(Level::Warn, ""));
    assert!(logger.rejected(Level::Info, ""));
    assert!(logger.rejected(Level::Debug, ""));
    assert!(logger.rejected(Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_info_lc() {
    let builder = RustLogFilterBuilder::from_spec("info").unwrap();
    let logger = builder.build();
    assert!(!logger.rejected(Level::Error, ""));
    assert!(!logger.rejected(Level::Warn, ""));
    assert!(!logger.rejected(Level::Info, ""));
    assert!(logger.rejected(Level::Debug, ""));
    assert!(logger.rejected(Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_info_uc() {
    let builder = RustLogFilterBuilder::from_spec("INFO").unwrap();
    let logger = builder.build();
    assert!(!logger.rejected(Level::Error, ""));
    assert!(!logger.rejected(Level::Warn, ""));
    assert!(!logger.rejected(Level::Info, ""));
    assert!(logger.rejected(Level::Debug, ""));
    assert!(logger.rejected(Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_debug_lc() {
    let builder = RustLogFilterBuilder::from_spec("debug").unwrap();
    let logger = builder.build();
    assert!(!logger.rejected(Level::Error, ""));
    assert!(!logger.rejected(Level::Warn, ""));
    assert!(!logger.rejected(Level::Info, ""));
    assert!(!logger.rejected(Level::Debug, ""));
    assert!(logger.rejected(Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_debug_uc() {
    let builder = RustLogFilterBuilder::from_spec("DEBUG").unwrap();
    let logger = builder.build();
    assert!(!logger.rejected(Level::Error, ""));
    assert!(!logger.rejected(Level::Warn, ""));
    assert!(!logger.rejected(Level::Info, ""));
    assert!(!logger.rejected(Level::Debug, ""));
    assert!(logger.rejected(Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_trace_lc() {
    let builder = RustLogFilterBuilder::from_spec("trace").unwrap();
    let logger = builder.build();
    assert!(!logger.rejected(Level::Error, ""));
    assert!(!logger.rejected(Level::Warn, ""));
    assert!(!logger.rejected(Level::Info, ""));
    assert!(!logger.rejected(Level::Debug, ""));
    assert!(!logger.rejected(Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_trace_uc() {
    let builder = RustLogFilterBuilder::from_spec("TRACE").unwrap();
    let logger = builder.build();
    assert!(!logger.rejected(Level::Error, ""));
    assert!(!logger.rejected(Level::Warn, ""));
    assert!(!logger.rejected(Level::Info, ""));
    assert!(!logger.rejected(Level::Debug, ""));
    assert!(!logger.rejected(Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_debug_mixed() {
    for spec in ["Debug", "debuG", "deBug", "DeBuG"] {
        let builder = RustLogFilterBuilder::from_spec(spec).unwrap();
        let logger = builder.build();
        assert!(!logger.rejected(Level::Error, ""));
        assert!(!logger.rejected(Level::Warn, ""));
        assert!(!logger.rejected(Level::Info, ""));
        assert!(!logger.rejected(Level::Debug, ""));
        assert!(logger.rejected(Level::Trace, ""));
    }
}

#[test]
fn from_spec_and_try_from_error_propagation() {
    let error = RustLogFilterBuilder::from_spec("info,crate1=invalid").unwrap_err();
    assert_snapshot!(error.to_string(), @"malformed logging spec 'invalid'");

    let filter = RustLogFilter::try_from("info,crate1::mod1=warn").unwrap();
    assert!(!filter.rejected(Level::Warn, "crate1::mod1"));
    assert!(filter.rejected(Level::Info, "crate1::mod1"));
    assert!(!filter.rejected(Level::Info, "crate2::mod2"));

    let err = RustLogFilter::try_from("info,crate1=invalid").unwrap_err();
    assert_snapshot!(err.to_string(), @"malformed logging spec 'invalid'");
}

#[test]
fn enabled_and_dual_mask_fast_paths() {
    // Global info, crate1::mod1 narrowed to warn, crate2::sub widened to debug.
    let builder =
        RustLogFilterBuilder::from_spec("info,crate1::mod1=warn,crate2::sub=debug").unwrap();
    let filter = builder.build();

    // Trace is disabled everywhere -> filter_level(Trace) is Reject.
    assert_eq!(filter.filter_level(Level::Trace), FilterResult::Reject);
    // Debug is enabled only for crate2::sub -> filter_level(Debug) is Neutral (not always_neutral).
    assert_eq!(filter.filter_level(Level::Debug), FilterResult::Neutral);
    // Info is disabled for crate1::mod1 -> filter_level(Info) is Neutral (not always_neutral).
    assert_eq!(filter.filter_level(Level::Info), FilterResult::Neutral);
    // Warn and Error are enabled everywhere -> always_neutral fast path returns Neutral even with
    // no module_path.
    assert_eq!(filter.filter_level(Level::Warn), FilterResult::Neutral);
    assert_eq!(filter.filter_level(Level::Error), FilterResult::Neutral);

    let no_module_warn = Record::builder().level(Level::Warn).build();
    let no_module_debug = Record::builder().level(Level::Debug).build();
    assert_eq!(filter.filter(&no_module_warn, &()), FilterResult::Neutral);
    assert_eq!(filter.filter(&no_module_debug, &()), FilterResult::Reject);
}

#[test]
fn logforth_target_kv_overrides_module_path_without_colliding_with_user_target_kv() {
    let builder = RustLogFilterBuilder::from_spec("info,my_app::query=warn").unwrap();
    let filter = builder.build();

    // Reserved `logforth.target` overrides `module_path`.
    let bridged_kvs = [(Key::new("logforth.target"), Value::str("my_app::query"))];
    let bridged_info = Record::builder()
        .level(Level::Info)
        .module_path_static("my_app::db::driver")
        .key_values(KeyValues::from(bridged_kvs.as_slice()))
        .build();
    assert_eq!(filter.filter(&bridged_info, &()), FilterResult::Reject);

    let bridged_warn = Record::builder()
        .level(Level::Warn)
        .module_path_static("my_app::db::driver")
        .key_values(KeyValues::from(bridged_kvs.as_slice()))
        .build();
    assert_eq!(filter.filter(&bridged_warn, &()), FilterResult::Neutral);

    // Application `target` KV does NOT collide with reserved `logforth.target`.
    let user_kvs = [(Key::new("target"), Value::str("my_app::query"))];
    let app_info = Record::builder()
        .level(Level::Info)
        .module_path_static("my_app::deploy")
        .key_values(KeyValues::from(user_kvs.as_slice()))
        .build();
    assert_eq!(filter.filter(&app_info, &()), FilterResult::Neutral);
}

#[test]
fn match_full_path() {
    let logger = RustLogFilter::from_directives(vec![
        Directive {
            prefix: Some("crate2".to_owned()),
            level: LevelFilter::MoreSevereEqual(Level::Info),
        },
        Directive {
            prefix: Some("crate1::mod1".to_owned()),
            level: LevelFilter::MoreSevereEqual(Level::Warn),
        },
    ]);
    assert!(!logger.rejected(Level::Warn, "crate1::mod1"));
    assert!(logger.rejected(Level::Info, "crate1::mod1"));
    assert!(!logger.rejected(Level::Info, "crate2"));
    assert!(logger.rejected(Level::Debug, "crate2"));
}

#[test]
fn no_match() {
    let logger = RustLogFilter::from_directives(vec![
        Directive {
            prefix: Some("crate2".to_owned()),
            level: LevelFilter::MoreSevereEqual(Level::Info),
        },
        Directive {
            prefix: Some("crate1::mod1".to_owned()),
            level: LevelFilter::MoreSevereEqual(Level::Warn),
        },
    ]);
    assert!(logger.rejected(Level::Warn, "crate3"));
}

#[test]
fn match_beginning() {
    let logger = RustLogFilter::from_directives(vec![
        Directive {
            prefix: Some("crate2".to_owned()),
            level: LevelFilter::MoreSevereEqual(Level::Info),
        },
        Directive {
            prefix: Some("crate1::mod1".to_owned()),
            level: LevelFilter::MoreSevereEqual(Level::Warn),
        },
    ]);
    assert!(!logger.rejected(Level::Info, "crate2::mod1"));
}

#[test]
fn match_beginning_longest_match() {
    let logger = RustLogFilter::from_directives(vec![
        Directive {
            prefix: Some("crate2".to_owned()),
            level: LevelFilter::MoreSevereEqual(Level::Info),
        },
        Directive {
            prefix: Some("crate2::mod".to_owned()),
            level: LevelFilter::MoreSevereEqual(Level::Debug),
        },
        Directive {
            prefix: Some("crate1::mod1".to_owned()),
            level: LevelFilter::MoreSevereEqual(Level::Warn),
        },
    ]);
    assert!(!logger.rejected(Level::Debug, "crate2::mod1"));
    assert!(logger.rejected(Level::Debug, "crate2"));
}

#[test]
fn match_default() {
    let logger = RustLogFilter::from_directives(vec![
        Directive {
            prefix: None,
            level: LevelFilter::MoreSevereEqual(Level::Info),
        },
        Directive {
            prefix: Some("crate1::mod1".to_owned()),
            level: LevelFilter::MoreSevereEqual(Level::Warn),
        },
    ]);
    assert!(!logger.rejected(Level::Warn, "crate1::mod1"));
    assert!(!logger.rejected(Level::Info, "crate2::mod2"));
}

#[test]
fn zero_level() {
    let logger = RustLogFilter::from_directives(vec![
        Directive {
            prefix: None,
            level: LevelFilter::MoreSevereEqual(Level::Info),
        },
        Directive {
            prefix: Some("crate1::mod1".to_owned()),
            level: LevelFilter::Off,
        },
    ]);
    assert!(logger.rejected(Level::Error, "crate1::mod1"));
    assert!(!logger.rejected(Level::Info, "crate2::mod2"));
}
