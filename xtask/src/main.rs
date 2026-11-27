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

use std::path::Path;
use std::process;

use clap::Parser;
use clap::Subcommand;

use crate::build::CommandBuild;
use crate::check::CommandCheck;
use crate::lint::CommandLint;
use crate::test::CommandTest;

mod build;
mod check;
mod lint;
mod test;

fn main() {
    Command::parse().run();
}

#[derive(Parser)]
struct Command {
    #[command(subcommand)]
    sub: SubCommand,
}

impl Command {
    fn run(self) {
        match self.sub {
            SubCommand::Build(cmd) => cmd.run(),
            SubCommand::Check(cmd) => cmd.run(),
            SubCommand::Lint(cmd) => cmd.run(),
            SubCommand::Test(cmd) => cmd.run(),
        }
    }
}

#[derive(Subcommand)]
enum SubCommand {
    #[command(about = "Compile workspace packages.")]
    Build(CommandBuild),
    #[command(about = "Check workspace packages across feature configurations.")]
    Check(CommandCheck),
    #[command(about = "Run workspace quality checks.")]
    Lint(CommandLint),
    #[command(about = "Run workspace tests.")]
    Test(CommandTest),
}

fn find_command(command: &str) -> process::Command {
    match which::which(command) {
        Ok(executable) => {
            let mut cmd = process::Command::new(executable);
            cmd.current_dir(Path::new(env!("CARGO_WORKSPACE_DIR")));
            cmd
        }
        Err(err) => panic!("{command} not found: {err}"),
    }
}

fn ensure_installed(binary: &str, crate_name: &str) {
    if which::which(binary).is_err() {
        let mut cmd = find_command("cargo");
        cmd.args(["install", crate_name]);
        run_command(cmd);
    }
}

fn run_command(mut cmd: process::Command) {
    println!("{cmd:?}");
    let status = cmd.status().expect("failed to execute process");
    assert!(status.success(), "command failed: {status}");
}
