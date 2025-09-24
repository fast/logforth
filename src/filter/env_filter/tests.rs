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

use snapbox::assert_data_eq;
use snapbox::str;

use super::*;

#[test]
fn parse_spec_valid() {
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec("crate1::mod1=error,crate1::mod2,crate2=debug");

    assert_eq!(dirs.len(), 3);
    assert_eq!(dirs[0].name, Some("crate1::mod1".to_owned()));
    assert_eq!(dirs[0].level, LevelFilter::Error);

    assert_eq!(dirs[1].name, Some("crate1::mod2".to_owned()));
    assert_eq!(dirs[1].level, LevelFilter::Trace);

    assert_eq!(dirs[2].name, Some("crate2".to_owned()));
    assert_eq!(dirs[2].level, LevelFilter::Debug);

    assert!(errors.is_empty());
}

#[test]
fn parse_spec_invalid_crate() {
    // test parse_spec with multiple = in specification
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec("crate1::mod1=warn=info,crate2=debug");

    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].name, Some("crate2".to_owned()));
    assert_eq!(dirs[0].level, LevelFilter::Debug);

    assert_eq!(errors.len(), 1);
    assert_data_eq!(
        &errors[0],
        str!["malformed logging spec 'crate1::mod1=warn=info'"]
    );
}

#[test]
fn parse_spec_invalid_level() {
    // test parse_spec with 'noNumber' as log level
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec("crate1::mod1=noNumber,crate2=debug");

    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].name, Some("crate2".to_owned()));
    assert_eq!(dirs[0].level, LevelFilter::Debug);

    assert_eq!(errors.len(), 1);
    assert_data_eq!(&errors[0], str!["malformed logging spec 'noNumber'"]);
}

#[test]
fn parse_spec_string_level() {
    // test parse_spec with 'warn' as log level
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec("crate1::mod1=wrong,crate2=warn");

    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].name, Some("crate2".to_owned()));
    assert_eq!(dirs[0].level, LevelFilter::Warn);

    assert_eq!(errors.len(), 1);
    assert_data_eq!(&errors[0], str!["malformed logging spec 'wrong'"]);
}

#[test]
fn parse_spec_empty_level() {
    // test parse_spec with '' as log level
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec("crate1::mod1=wrong,crate2=");

    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].name, Some("crate2".to_owned()));
    assert_eq!(dirs[0].level, LevelFilter::Trace);

    assert_eq!(errors.len(), 1);
    assert_data_eq!(&errors[0], str!["malformed logging spec 'wrong'"]);
}

#[test]
fn parse_spec_empty_level_isolated() {
    // test parse_spec with "" as log level (and the entire spec str)
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec(""); // should be ignored
    assert_eq!(dirs.len(), 0);
    assert!(errors.is_empty());
}

#[test]
fn parse_spec_blank_level_isolated() {
    // test parse_spec with a white-space-only string specified as the log
    // level (and the entire spec str)
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec("     "); // should be ignored
    assert_eq!(dirs.len(), 0);
    assert!(errors.is_empty());
}

#[test]
fn parse_spec_blank_level_isolated_comma_only() {
    // The spec should contain zero or more comma-separated string slices,
    // so a comma-only string should be interpreted as two empty strings
    // (which should both be treated as invalid, so ignored).
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec(","); // should be ignored
    assert_eq!(dirs.len(), 0);
    assert!(errors.is_empty());
}

#[test]
fn parse_spec_blank_level_isolated_comma_blank() {
    // The spec should contain zero or more comma-separated string slices,
    // so this bogus spec should be interpreted as containing one empty
    // string and one blank string. Both should both be treated as
    // invalid, so ignored.
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec(",     "); // should be ignored
    assert_eq!(dirs.len(), 0);

    assert!(errors.is_empty());
}

#[test]
fn parse_spec_blank_level_isolated_blank_comma() {
    // The spec should contain zero or more comma-separated string slices,
    // so this bogus spec should be interpreted as containing one blank
    // string and one empty string. Both should both be treated as
    // invalid, so ignored.
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec("     ,"); // should be ignored
    assert_eq!(dirs.len(), 0);

    assert!(errors.is_empty());
}

#[test]
fn parse_spec_global() {
    // test parse_spec with no crate
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec("warn,crate2=debug");
    assert_eq!(dirs.len(), 2);
    assert_eq!(dirs[0].name, None);
    assert_eq!(dirs[0].level, LevelFilter::Warn);
    assert_eq!(dirs[1].name, Some("crate2".to_owned()));
    assert_eq!(dirs[1].level, LevelFilter::Debug);

    assert!(errors.is_empty());
}

#[test]
fn parse_spec_global_bare_warn_lc() {
    // test parse_spec with no crate, in isolation, all lowercase
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec("warn");
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].name, None);
    assert_eq!(dirs[0].level, LevelFilter::Warn);

    assert!(errors.is_empty());
}

#[test]
fn parse_spec_global_bare_warn_uc() {
    // test parse_spec with no crate, in isolation, all uppercase
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec("WARN");
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].name, None);
    assert_eq!(dirs[0].level, LevelFilter::Warn);

    assert!(errors.is_empty());
}

#[test]
fn parse_spec_global_bare_warn_mixed() {
    // test parse_spec with no crate, in isolation, mixed case
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec("wArN");
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].name, None);
    assert_eq!(dirs[0].level, LevelFilter::Warn);

    assert!(errors.is_empty());
}

#[test]
fn parse_spec_multiple_invalid_crates() {
    // test parse_spec with multiple = in specification
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec("crate1::mod1=warn=info,crate2=debug,crate3=error=error");

    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].name, Some("crate2".to_owned()));
    assert_eq!(dirs[0].level, LevelFilter::Debug);

    assert_eq!(errors.len(), 2);
    assert_data_eq!(
        &errors[0],
        str!["malformed logging spec 'crate1::mod1=warn=info'"]
    );
    assert_data_eq!(
        &errors[1],
        str!["malformed logging spec 'crate3=error=error'"]
    );
}

#[test]
fn parse_spec_multiple_invalid_levels() {
    // test parse_spec with 'noNumber' as log level
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec("crate1::mod1=noNumber,crate2=debug,crate3=invalid");

    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].name, Some("crate2".to_owned()));
    assert_eq!(dirs[0].level, LevelFilter::Debug);

    assert_eq!(errors.len(), 2);
    assert_data_eq!(&errors[0], str!["malformed logging spec 'noNumber'"]);
    assert_data_eq!(&errors[1], str!["malformed logging spec 'invalid'"]);
}

#[test]
fn parse_spec_invalid_crate_and_level() {
    // test parse_spec with 'noNumber' as log level
    let ParseResult {
        directives: dirs,
        errors,
    } = parse_spec("crate1::mod1=debug=info,crate2=debug,crate3=invalid");

    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].name, Some("crate2".to_owned()));
    assert_eq!(dirs[0].level, LevelFilter::Debug);

    assert_eq!(errors.len(), 2);
    assert_data_eq!(
        &errors[0],
        str!["malformed logging spec 'crate1::mod1=debug=info'"]
    );
    assert_data_eq!(&errors[1], str!["malformed logging spec 'invalid'"]);
}

#[test]
fn parse_error_message_single_error() {
    let ParseResult { errors, .. } = parse_spec("crate1::mod1=debug=info,crate2=debug");
    assert_data_eq!(
        &errors[0],
        str!["malformed logging spec 'crate1::mod1=debug=info'"]
    );
}

#[test]
fn parse_error_message_multiple_errors() {
    let ParseResult { errors, .. } =
        parse_spec("crate1::mod1=debug=info,crate2=debug,crate3=invalid");
    assert_data_eq!(
        &errors[0],
        str!["malformed logging spec 'crate1::mod1=debug=info'"]
    );
}

fn make_logger_filter(dirs: Vec<Directive>) -> EnvFilter {
    let mut logger = EnvFilterBuilder::default().build();
    logger.directives = dirs;
    logger
}

#[test]
fn filter_info() {
    let logger = EnvFilterBuilder::default()
        .filter(None, LevelFilter::Info)
        .build();
    assert!(enabled(&logger.directives, Level::Info, "crate1"));
    assert!(!enabled(&logger.directives, Level::Debug, "crate1"));
}

#[test]
fn filter_beginning_longest_match() {
    let logger = EnvFilterBuilder::default()
        .filter(Some("crate2"), LevelFilter::Info)
        .filter(Some("crate2::mod"), LevelFilter::Debug)
        .filter(Some("crate1::mod1"), LevelFilter::Warn)
        .build();
    assert!(enabled(&logger.directives, Level::Debug, "crate2::mod1"));
    assert!(!enabled(&logger.directives, Level::Debug, "crate2"));
}

// Some of our tests are only correct or complete when they cover the full
// universe of variants for log::Level. In the unlikely event that a new
// variant is added in the future, this test will detect the scenario and
// alert us to the need to review and update the tests. In such a
// situation, this test will fail to compile, and the error message will
// look something like this:
//
//     error[E0004]: non-exhaustive patterns: `NewVariant` not covered
//        --> src/filter/mod.rs:413:15
//         |
//     413 |         match level_universe {
//         |               ^^^^^^^^^^^^^^ pattern `NewVariant` not covered
#[test]
fn ensure_tests_cover_level_universe() {
    let level_universe: Level = Level::Trace; // use of trace variant is arbitrary
    match level_universe {
        Level::Error | Level::Warn | Level::Info | Level::Debug | Level::Trace => (),
    }
}

#[test]
fn parse_default() {
    let logger = EnvFilterBuilder::from_spec("info,crate1::mod1=warn").build();
    assert!(enabled(&logger.directives, Level::Warn, "crate1::mod1"));
    assert!(enabled(&logger.directives, Level::Info, "crate2::mod2"));
}

#[test]
fn parse_default_bare_level_off_lc() {
    let logger = EnvFilterBuilder::from_spec("off").build();
    assert!(!enabled(&logger.directives, Level::Error, ""));
    assert!(!enabled(&logger.directives, Level::Warn, ""));
    assert!(!enabled(&logger.directives, Level::Info, ""));
    assert!(!enabled(&logger.directives, Level::Debug, ""));
    assert!(!enabled(&logger.directives, Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_off_uc() {
    let logger = EnvFilterBuilder::from_spec("OFF").build();
    assert!(!enabled(&logger.directives, Level::Error, ""));
    assert!(!enabled(&logger.directives, Level::Warn, ""));
    assert!(!enabled(&logger.directives, Level::Info, ""));
    assert!(!enabled(&logger.directives, Level::Debug, ""));
    assert!(!enabled(&logger.directives, Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_error_lc() {
    let logger = EnvFilterBuilder::from_spec("error").build();
    assert!(enabled(&logger.directives, Level::Error, ""));
    assert!(!enabled(&logger.directives, Level::Warn, ""));
    assert!(!enabled(&logger.directives, Level::Info, ""));
    assert!(!enabled(&logger.directives, Level::Debug, ""));
    assert!(!enabled(&logger.directives, Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_error_uc() {
    let logger = EnvFilterBuilder::from_spec("ERROR").build();
    assert!(enabled(&logger.directives, Level::Error, ""));
    assert!(!enabled(&logger.directives, Level::Warn, ""));
    assert!(!enabled(&logger.directives, Level::Info, ""));
    assert!(!enabled(&logger.directives, Level::Debug, ""));
    assert!(!enabled(&logger.directives, Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_warn_lc() {
    let logger = EnvFilterBuilder::from_spec("warn").build();
    assert!(enabled(&logger.directives, Level::Error, ""));
    assert!(enabled(&logger.directives, Level::Warn, ""));
    assert!(!enabled(&logger.directives, Level::Info, ""));
    assert!(!enabled(&logger.directives, Level::Debug, ""));
    assert!(!enabled(&logger.directives, Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_warn_uc() {
    let logger = EnvFilterBuilder::from_spec("WARN").build();
    assert!(enabled(&logger.directives, Level::Error, ""));
    assert!(enabled(&logger.directives, Level::Warn, ""));
    assert!(!enabled(&logger.directives, Level::Info, ""));
    assert!(!enabled(&logger.directives, Level::Debug, ""));
    assert!(!enabled(&logger.directives, Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_info_lc() {
    let logger = EnvFilterBuilder::from_spec("info").build();
    assert!(enabled(&logger.directives, Level::Error, ""));
    assert!(enabled(&logger.directives, Level::Warn, ""));
    assert!(enabled(&logger.directives, Level::Info, ""));
    assert!(!enabled(&logger.directives, Level::Debug, ""));
    assert!(!enabled(&logger.directives, Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_info_uc() {
    let logger = EnvFilterBuilder::from_spec("INFO").build();
    assert!(enabled(&logger.directives, Level::Error, ""));
    assert!(enabled(&logger.directives, Level::Warn, ""));
    assert!(enabled(&logger.directives, Level::Info, ""));
    assert!(!enabled(&logger.directives, Level::Debug, ""));
    assert!(!enabled(&logger.directives, Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_debug_lc() {
    let logger = EnvFilterBuilder::from_spec("debug").build();
    assert!(enabled(&logger.directives, Level::Error, ""));
    assert!(enabled(&logger.directives, Level::Warn, ""));
    assert!(enabled(&logger.directives, Level::Info, ""));
    assert!(enabled(&logger.directives, Level::Debug, ""));
    assert!(!enabled(&logger.directives, Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_debug_uc() {
    let logger = EnvFilterBuilder::from_spec("DEBUG").build();
    assert!(enabled(&logger.directives, Level::Error, ""));
    assert!(enabled(&logger.directives, Level::Warn, ""));
    assert!(enabled(&logger.directives, Level::Info, ""));
    assert!(enabled(&logger.directives, Level::Debug, ""));
    assert!(!enabled(&logger.directives, Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_trace_lc() {
    let logger = EnvFilterBuilder::from_spec("trace").build();
    assert!(enabled(&logger.directives, Level::Error, ""));
    assert!(enabled(&logger.directives, Level::Warn, ""));
    assert!(enabled(&logger.directives, Level::Info, ""));
    assert!(enabled(&logger.directives, Level::Debug, ""));
    assert!(enabled(&logger.directives, Level::Trace, ""));
}

#[test]
fn parse_default_bare_level_trace_uc() {
    let logger = EnvFilterBuilder::from_spec("TRACE").build();
    assert!(enabled(&logger.directives, Level::Error, ""));
    assert!(enabled(&logger.directives, Level::Warn, ""));
    assert!(enabled(&logger.directives, Level::Info, ""));
    assert!(enabled(&logger.directives, Level::Debug, ""));
    assert!(enabled(&logger.directives, Level::Trace, ""));
}

// In practice, the desired log level is typically specified by a token
// that is either all lowercase (e.g., 'trace') or all uppercase (.e.g,
// 'TRACE'), but this tests serves as a reminder that
// log::Level::from_str() ignores all case variants.
#[test]
fn parse_default_bare_level_debug_mixed() {
    {
        let logger = EnvFilterBuilder::from_spec("Debug").build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }
    {
        let logger = EnvFilterBuilder::from_spec("debuG").build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }
    {
        let logger = EnvFilterBuilder::from_spec("deBug").build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }
    {
        let logger = EnvFilterBuilder::from_spec("DeBuG").build(); // LaTeX flavor!
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }
}

#[test]
fn try_parse_valid_filter() {
    let logger = EnvFilterBuilder::try_from_spec("info,crate1::mod1=warn")
        .expect("valid filter returned error")
        .build();
    assert!(enabled(&logger.directives, Level::Warn, "crate1::mod1"));
    assert!(enabled(&logger.directives, Level::Info, "crate2::mod2"));
}

#[test]
fn try_parse_invalid_filter() {
    let error = EnvFilterBuilder::try_from_spec("info,crate1=invalid").unwrap_err();
    assert_data_eq!(error, str!["malformed logging spec 'invalid'"]);
}

#[test]
fn match_full_path() {
    let logger = make_logger_filter(vec![
        Directive {
            name: Some("crate2".to_owned()),
            level: LevelFilter::Info,
        },
        Directive {
            name: Some("crate1::mod1".to_owned()),
            level: LevelFilter::Warn,
        },
    ]);
    assert!(enabled(&logger.directives, Level::Warn, "crate1::mod1"));
    assert!(!enabled(&logger.directives, Level::Info, "crate1::mod1"));
    assert!(enabled(&logger.directives, Level::Info, "crate2"));
    assert!(!enabled(&logger.directives, Level::Debug, "crate2"));
}

#[test]
fn no_match() {
    let logger = make_logger_filter(vec![
        Directive {
            name: Some("crate2".to_owned()),
            level: LevelFilter::Info,
        },
        Directive {
            name: Some("crate1::mod1".to_owned()),
            level: LevelFilter::Warn,
        },
    ]);
    assert!(!enabled(&logger.directives, Level::Warn, "crate3"));
}

#[test]
fn match_beginning() {
    let logger = make_logger_filter(vec![
        Directive {
            name: Some("crate2".to_owned()),
            level: LevelFilter::Info,
        },
        Directive {
            name: Some("crate1::mod1".to_owned()),
            level: LevelFilter::Warn,
        },
    ]);
    assert!(enabled(&logger.directives, Level::Info, "crate2::mod1"));
}

#[test]
fn match_beginning_longest_match() {
    let logger = make_logger_filter(vec![
        Directive {
            name: Some("crate2".to_owned()),
            level: LevelFilter::Info,
        },
        Directive {
            name: Some("crate2::mod".to_owned()),
            level: LevelFilter::Debug,
        },
        Directive {
            name: Some("crate1::mod1".to_owned()),
            level: LevelFilter::Warn,
        },
    ]);
    assert!(enabled(&logger.directives, Level::Debug, "crate2::mod1"));
    assert!(!enabled(&logger.directives, Level::Debug, "crate2"));
}

#[test]
fn match_default() {
    let logger = make_logger_filter(vec![
        Directive {
            name: None,
            level: LevelFilter::Info,
        },
        Directive {
            name: Some("crate1::mod1".to_owned()),
            level: LevelFilter::Warn,
        },
    ]);
    assert!(enabled(&logger.directives, Level::Warn, "crate1::mod1"));
    assert!(enabled(&logger.directives, Level::Info, "crate2::mod2"));
}

#[test]
fn zero_level() {
    let logger = make_logger_filter(vec![
        Directive {
            name: None,
            level: LevelFilter::Info,
        },
        Directive {
            name: Some("crate1::mod1".to_owned()),
            level: LevelFilter::Off,
        },
    ]);
    assert!(!enabled(&logger.directives, Level::Error, "crate1::mod1"));
    assert!(enabled(&logger.directives, Level::Info, "crate2::mod2"));
}
