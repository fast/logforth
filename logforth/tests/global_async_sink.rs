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

//! This case ensures that the asynchronous logger flushes correctly at program exit.

// This refers to https://github.com/SpriteOvO/spdlog-rs/issues/64

use std::fmt::Write;
use std::os::raw::c_int;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use logforth_append_async::AsyncBuilder;
use logforth_core::Append;
use logforth_core::Diagnostic;
use logforth_core::Error;
use logforth_core::record::LevelFilter;
use logforth_core::record::Record;

static IS_LOGGED: AtomicBool = AtomicBool::new(false);
static IS_FLUSHED: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
struct SetFlags;

impl Append for SetFlags {
    fn append(&self, _: &Record, _: &[Box<dyn Diagnostic>]) -> Result<(), Error> {
        IS_LOGGED.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> {
        // assert that the record has been logged before flushing
        assert!(IS_LOGGED.load(Ordering::SeqCst));
        IS_FLUSHED.store(true, Ordering::SeqCst);
        Ok(())
    }
}

fn run_test() {
    {
        extern "C" fn check() {
            // assert that `Async` appender in the default logger will be flushed correctly
            // and will not panic.
            assert!(IS_FLUSHED.load(Ordering::SeqCst));
        }

        // set up `atexit` to check the flag at the end of the program
        unsafe extern "C" {
            fn atexit(cb: extern "C" fn()) -> c_int;
        }

        assert_eq!(unsafe { atexit(check) }, 0);

        let asynchronous = AsyncBuilder::new("async-appender").append(SetFlags).build();

        logforth::starter_log::builder()
            .dispatch(|d| d.filter(LevelFilter::All).append(asynchronous))
            .apply();
    }

    log::info!("hello async sink");
}

fn main() {
    // This is a flaky test, it only has a certain probability of failing,
    // so we run it multiple times to make sure it's really working properly.
    {
        let mut captured_output = String::new();
        let args = std::env::args().collect::<Vec<_>>();

        let is_parent = args.iter().all(|arg| arg != "child");
        if is_parent {
            for i in 0..1000 {
                let output = std::process::Command::new(&args[0])
                    .arg("child")
                    .stderr(std::process::Stdio::piped())
                    .output()
                    .unwrap();

                let success = output.status.success();
                writeln!(
                    captured_output,
                    "Attempt #{i} = {}",
                    if success { "ok" } else { "failed!" }
                )
                .unwrap();

                if !success {
                    eprintln!("{captured_output}");

                    let stderr = String::from_utf8_lossy(&output.stderr).lines().fold(
                        String::new(),
                        |mut contents, line| {
                            writeln!(&mut contents, "> {line}").unwrap();
                            contents
                        },
                    );

                    eprintln!("stderr of the failed attempt:\n{stderr}");
                    panic!("test failed");
                }
            }
            return;
        } else {
            assert_eq!(args[1], "child");
        }
    }

    // Run the test after leaving the scope, so the main function ends
    // without dropping additional variables, thus exiting faster. This
    // should increase the probability of reproducing the error.
    run_test();
}
