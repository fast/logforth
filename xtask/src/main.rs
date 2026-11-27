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

#[derive(Parser)]
struct CommandBuild {
    #[arg(long, help = "Assert that `Cargo.lock` will remain unchanged.")]
    locked: bool,
}

impl CommandBuild {
    fn run(self) {
        run_command(make_build_cmd(self.locked));
    }
}

#[derive(Parser)]
struct CommandCheck;

impl CommandCheck {
    fn run(self) {
        for args in CHECK_CONFIGS {
            run_command(make_check_cmd(args));
        }
    }
}

#[derive(Parser)]
struct CommandTest {
    #[arg(long, help = "Do not capture test output.")]
    no_capture: bool,
}

impl CommandTest {
    fn run(self) {
        run_command(make_test_cmd(self.no_capture));
    }
}

#[derive(Parser)]
#[command(name = "lint")]
struct CommandLint {
    #[arg(long, help = "Automatically apply lint and formatting suggestions.")]
    fix: bool,
}

impl CommandLint {
    fn run(self) {
        run_command(make_clippy_cmd(self.fix));
        run_command(make_format_cmd(self.fix));
        run_command(make_taplo_cmd(self.fix));
        run_command(make_typos_cmd());
        run_command(make_hawkeye_cmd(self.fix));
        if !self.fix {
            run_command(make_doc_cmd());
        }
    }
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
        let mut cmd = cargo();
        cmd.args(["install", crate_name]);
        run_command(cmd);
    }
}

fn cargo() -> process::Command {
    find_command("cargo")
}

fn run_command(mut cmd: process::Command) {
    println!("{cmd:?}");
    let status = cmd.status().expect("failed to execute process");
    assert!(status.success(), "command failed: {status}");
}

fn make_build_cmd(locked: bool) -> process::Command {
    let mut cmd = cargo();
    cmd.args(["build", "--workspace", "--all-targets", "--all-features"]);
    if locked {
        cmd.arg("--locked");
    }
    cmd
}

fn make_check_cmd(extra_args: &[&str]) -> process::Command {
    let mut cmd = cargo();
    cmd.env("RUSTFLAGS", "-D warnings");
    cmd.args(["check", "--all-targets"]);
    cmd.args(extra_args);
    cmd
}

fn make_test_cmd(no_capture: bool) -> process::Command {
    let mut cmd = cargo();
    cmd.args(["test", "--workspace", "--all-features"]);
    if no_capture {
        cmd.args(["--", "--nocapture"]);
    }
    cmd
}

fn make_format_cmd(fix: bool) -> process::Command {
    let mut cmd = cargo();
    cmd.args(["+nightly", "fmt", "--all"]);
    if !fix {
        cmd.args(["--", "--check"]);
    }
    cmd
}

fn make_clippy_cmd(fix: bool) -> process::Command {
    let mut cmd = cargo();
    cmd.args([
        "+nightly",
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
    ]);
    if fix {
        cmd.args(["--allow-staged", "--allow-dirty", "--fix"]);
    } else {
        cmd.args(["--", "-D", "warnings"]);
    }
    cmd
}

fn make_doc_cmd() -> process::Command {
    let mut cmd = cargo();
    cmd.env("RUSTDOCFLAGS", "-D warnings --cfg docsrs");
    cmd.args([
        "+nightly",
        "doc",
        "--workspace",
        "--all-features",
        "--no-deps",
    ]);
    cmd
}

fn make_hawkeye_cmd(fix: bool) -> process::Command {
    ensure_installed("hawkeye", "hawkeye");
    let mut cmd = find_command("hawkeye");
    if fix {
        cmd.arg("format");
    } else {
        cmd.arg("check");
    }
    cmd
}

fn make_typos_cmd() -> process::Command {
    ensure_installed("typos", "typos-cli");
    find_command("typos")
}

fn make_taplo_cmd(fix: bool) -> process::Command {
    ensure_installed("taplo", "taplo-cli");
    let mut cmd = find_command("taplo");
    if fix {
        cmd.arg("format");
    } else {
        cmd.args(["format", "--check"]);
    }
    cmd
}

fn main() {
    Command::parse().run();
}
