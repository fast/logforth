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

fn main() {
    println!(
        r#"Run this example with:

1. `cargo test --example testing -- --show-output`
2. `cargo test --example testing -- --nocapture`
3. `cargo test --example testing`

Compare the output of the three commands."#
    );
}

#[cfg(test)]
mod tests {
    use logforth::LevelFilter;
    use logforth::append::Testing;

    #[test]
    fn testing() {
        logforth::builder()
            .dispatch(|d| d.filter(LevelFilter::Trace).append(Testing::default()))
            .setup_log_crate();

        log::error!("Hello error!");
        log::warn!("Hello warn!");
        log::info!("Hello info!");
        log::debug!("Hello debug!");
        log::trace!("Hello trace!");
    }
}
