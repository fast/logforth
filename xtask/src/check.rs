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

use std::process;

use clap::Parser;

use crate::find_command;
use crate::run_command;

const CHECK_CONFIGS: &[&[&str]] = &[
    &["--package", "logforth-core", "--no-default-features"],
    &["--package", "logforth-core", "--all-features"],
    &["--package", "logforth-bridge-log", "--no-default-features"],
    &["--package", "logforth-bridge-log", "--all-features"],
    &[
        "--package",
        "logforth-append-syslog",
        "--no-default-features",
    ],
    &["--package", "logforth-append-syslog", "--all-features"],
    &["--package", "logforth", "--no-default-features"],
    &[
        "--package",
        "logforth",
        "--no-default-features",
        "--features",
        "starter-log",
    ],
    &["--workspace", "--all-features"],
];

#[derive(Parser)]
pub struct CommandCheck;

impl CommandCheck {
    pub fn run(self) {
        for args in CHECK_CONFIGS {
            run_command(make_check_cmd(args));
        }
    }
}

fn make_check_cmd(extra_args: &[&str]) -> process::Command {
    let mut cmd = find_command("cargo");
    cmd.env("RUSTFLAGS", "-D warnings");
    cmd.args(["+nightly", "check", "--all-targets"]);
    cmd.args(extra_args);
    cmd
}
