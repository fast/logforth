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

use std::io::Write as _;

use logforth_core::kv::ValueView;

pub enum FieldName<'a> {
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
/// See [`crate::Journald`] for the rules.
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

pub trait PutAsFieldValue {
    fn put_field_value(self, buffer: &mut Vec<u8>);
}

impl PutAsFieldValue for &[u8] {
    fn put_field_value(self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(self)
    }
}

impl PutAsFieldValue for &str {
    fn put_field_value(self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(self.as_bytes())
    }
}

impl PutAsFieldValue for std::fmt::Arguments<'_> {
    fn put_field_value(self, buffer: &mut Vec<u8>) {
        // SAFETY: no more than an allocate-less version
        //  buffer.extend_from_slice(format!("{}", self))
        write!(buffer, "{self}").unwrap();
    }
}

impl PutAsFieldValue for ValueView<'_> {
    fn put_field_value(self, buffer: &mut Vec<u8>) {
        // SAFETY: no more than an allocate-less version
        //  buffer.extend_from_slice(format!("{}", self))
        write!(buffer, "{self}").unwrap();
    }
}

pub fn put_field_length_encoded<V: PutAsFieldValue>(
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

pub fn put_field_bytes(buffer: &mut Vec<u8>, name: FieldName<'_>, value: &[u8]) {
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
