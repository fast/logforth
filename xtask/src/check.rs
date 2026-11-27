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

use cargo_metadata::MetadataCommand;
use clap::Parser;

use crate::cargo;
use crate::run_command;

#[derive(Parser)]
pub struct CommandCheck;

impl CommandCheck {
    pub fn run(self) {
        for (package, features) in packages_with_features() {
            run_command(make_package_check_cmd(&package, &[]));
            for feature in features.chunks(1) {
                run_command(make_package_check_cmd(&package, feature));
            }
            if features.len() > 1 {
                run_command(make_package_check_cmd(&package, &features));
            }
        }
        run_command(make_workspace_check_cmd());
    }
}

fn packages_with_features() -> Vec<(String, Vec<String>)> {
    let manifest = Path::new(env!("CARGO_WORKSPACE_DIR")).join("Cargo.toml");
    let metadata = MetadataCommand::new()
        .manifest_path(manifest)
        .no_deps()
        .exec()
        .expect("failed to get cargo metadata");

    let mut packages = Vec::new();
    for package in metadata.workspace_packages() {
        let mut features = package
            .features
            .keys()
            .filter(|feature| feature.as_str() != "default")
            .cloned()
            .collect::<Vec<_>>();
        if features.is_empty() {
            continue;
        }
        features.sort();
        packages.push((package.name.to_string(), features));
    }
    packages.sort_by(|(lhs, _), (rhs, _)| lhs.cmp(rhs));
    packages
}

fn make_package_check_cmd(package: &str, features: &[String]) -> process::Command {
    let mut cmd = cargo();
    cmd.env("RUSTFLAGS", "-Dwarnings");
    cmd.args([
        "+nightly",
        "check",
        "--package",
        package,
        "--all-targets",
        "--no-default-features",
    ]);
    for feature in features {
        cmd.args(["--features", feature]);
    }
    cmd
}

fn make_workspace_check_cmd() -> process::Command {
    let mut cmd = cargo();
    cmd.env("RUSTFLAGS", "-Dwarnings");
    cmd.args([
        "+nightly",
        "check",
        "--workspace",
        "--exclude",
        "x",
        "--all-targets",
        "--all-features",
    ]);
    cmd
}
