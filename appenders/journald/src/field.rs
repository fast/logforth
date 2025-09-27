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

// This field is derived from https://github.com/swsnr/systemd-journal-logger.rs/blob/v2.2.0/src/fields.rs.

//! Write well-formatted journal fields to buffers.

use std::fmt::Arguments;
use std::io::Write;

use logforth_core::kv::Value;

pub(super) enum FieldName<'a> {
    WellFormed(&'a str),
    WriteEscaped(&'a str),
}

/// Whether `c` is a valid character in the key of a journal field.
///
/// Journal field keys may only contain ASCII uppercase letters A to Z,
/// numbers 0 to 9 and the underscore.
fn is_valid_key_char(c: char) -> bool {
    matches!(c, 'A'..='Z' | '0'..='9' | '_')
}

/// Write an escaped `key` for use in a systemd journal field.
///
/// See [`super::Journald`] for the rules.
fn write_escaped_key(key: &str, buffer: &mut Vec<u8>) {
    // Key length is limited to 64 bytes
    let mut remaining = 64;

    let escaped = key
        .to_ascii_uppercase()
        .replace(|c| !is_valid_key_char(c), "_");

    if escaped.starts_with(|c: char| matches!(c, '_' | '0'..='9')) {
        buffer.extend_from_slice(b"ESCAPED_");
        remaining -= 8;
    }

    for b in escaped.into_bytes() {
        if remaining == 0 {
            break;
        }
        buffer.push(b);
        remaining -= 1;
    }
}

fn put_field_name(buffer: &mut Vec<u8>, name: FieldName<'_>) {
    match name {
        FieldName::WellFormed(name) => buffer.extend_from_slice(name.as_bytes()),
        FieldName::WriteEscaped("") => buffer.extend_from_slice(b"EMPTY"),
        FieldName::WriteEscaped(name) => write_escaped_key(name, buffer),
    }
}

pub(super) trait PutAsFieldValue {
    fn put_field_value(self, buffer: &mut Vec<u8>);
}

impl PutAsFieldValue for &[u8] {
    fn put_field_value(self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(self)
    }
}

impl PutAsFieldValue for &Arguments<'_> {
    fn put_field_value(self, buffer: &mut Vec<u8>) {
        match self.as_str() {
            Some(s) => buffer.extend_from_slice(s.as_bytes()),
            None => {
                // SAFETY: no more than an allocate-less version
                //  buffer.extend_from_slice(format!("{}", self))
                write!(buffer, "{self}").unwrap()
            }
        }
    }
}

impl PutAsFieldValue for Value<'_> {
    fn put_field_value(self, buffer: &mut Vec<u8>) {
        // SAFETY: no more than an allocate-less version
        //  buffer.extend_from_slice(format!("{}", self))
        write!(buffer, "{self}").unwrap();
    }
}

pub(super) fn put_field_length_encoded<V: PutAsFieldValue>(
    buffer: &mut Vec<u8>,
    name: FieldName<'_>,
    value: V,
) {
    put_field_name(buffer, name);
    buffer.push(b'\n');
    // Reserve the length tag
    buffer.extend_from_slice(&[0; 8]);
    let value_start = buffer.len();
    value.put_field_value(buffer);
    let value_end = buffer.len();
    // Fill the length tag
    let length_bytes = ((value_end - value_start) as u64).to_le_bytes();
    buffer[value_start - 8..value_start].copy_from_slice(&length_bytes);
    buffer.push(b'\n');
}

pub(super) fn put_field_bytes(buffer: &mut Vec<u8>, name: FieldName<'_>, value: &[u8]) {
    if value.contains(&b'\n') {
        // Write as length encoded field
        put_field_length_encoded(buffer, name, value);
    } else {
        put_field_name(buffer, name);
        buffer.push(b'=');
        buffer.extend_from_slice(value);
        buffer.push(b'\n');
    }
}

#[cfg(test)]
mod tests {
    use FieldName::*;

    use super::*;

    #[test]
    fn test_escape_journal_key() {
        for case in ["FOO", "FOO_123"] {
            let mut bs = vec![];
            write_escaped_key(case, &mut bs);
            assert_eq!(String::from_utf8_lossy(&bs), case);
        }

        let cases = vec![
            ("foo", "FOO"),
            ("_foo", "ESCAPED__FOO"),
            ("1foo", "ESCAPED_1FOO"),
            ("Hallöchen", "HALL_CHEN"),
        ];
        for (key, expected) in cases {
            let mut bs = vec![];
            write_escaped_key(key, &mut bs);
            assert_eq!(String::from_utf8_lossy(&bs), expected);
        }

        {
            for case in [
                "very_long_key_name_that_is_longer_than_64_bytes".repeat(5),
                "_need_escape_very_long_key_name_that_is_longer_than_64_bytes".repeat(5),
            ] {
                let mut bs = vec![];
                write_escaped_key(&case, &mut bs);
                println!("{:?}", String::from_utf8_lossy(&bs));
                assert_eq!(bs.len(), 64);
            }
        }
    }

    #[test]
    fn test_put_field_length_encoded() {
        let mut buffer = Vec::new();
        // See "Data Format" in https://systemd.io/JOURNAL_NATIVE_PROTOCOL/ for this example
        put_field_length_encoded(&mut buffer, WellFormed("FOO"), "BAR".as_bytes());
        assert_eq!(&buffer, b"FOO\n\x03\0\0\0\0\0\0\0BAR\n");
    }

    #[test]
    fn test_put_field_bytes_no_newline() {
        let mut buffer = Vec::new();
        put_field_bytes(&mut buffer, WellFormed("FOO"), "BAR".as_bytes());
        assert_eq!(&buffer, b"FOO=BAR\n");
    }

    #[test]
    fn test_put_field_bytes_newline() {
        let mut buffer = Vec::new();
        put_field_bytes(
            &mut buffer,
            WellFormed("FOO"),
            "BAR\nSPAM_WITH_EGGS".as_bytes(),
        );
        assert_eq!(&buffer, b"FOO\n\x12\0\0\0\0\0\0\0BAR\nSPAM_WITH_EGGS\n");
    }
}
