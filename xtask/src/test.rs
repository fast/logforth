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

use crate::cargo;
use crate::run_command;

#[derive(Parser)]
pub struct CommandTest {
    #[arg(long, help = "Do not capture test output.")]
    no_capture: bool,
}

impl CommandTest {
    pub fn run(self) {
        run_command(make_test_cmd(self.no_capture));
    }
}

fn make_test_cmd(no_capture: bool) -> process::Command {
    let mut cmd = cargo();
    cmd.args(["test", "--workspace", "--exclude", "x", "--all-features"]);
    if no_capture {
        cmd.args(["--", "--nocapture"]);
    }
    cmd
}
