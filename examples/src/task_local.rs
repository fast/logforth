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

use logforth::diagnostic::TaskLocalDiagnostic;
use logforth::diagnostic::task_local::FutureExt;
use logforth::layout::TextLayout;
use logforth::record::LevelFilter;

#[tokio::main]
async fn main() {
    logforth::starter_log::builder()
        .dispatch(|d| {
            d.filter(LevelFilter::All)
                .diagnostic(TaskLocalDiagnostic::default())
                .append(logforth::append::Stderr::default().with_layout(TextLayout::default()))
        })
        .apply();

    async {
        async {
            log::error!("Hello error!");
            log::warn!("Hello warn!");
            log::info!("Hello info!");
        }
        .with_task_local_context([("k3".to_string(), "v3".to_string())])
        .await;
        log::debug!("Hello debug!");
        log::trace!("Hello trace!");
    }
    .with_task_local_context([("k1".to_string(), "v1".to_string())])
    .with_task_local_context([("k2".to_string(), "v2".to_string())])
    .await;
}
