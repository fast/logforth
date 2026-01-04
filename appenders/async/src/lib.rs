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

//! A composable appender, logging and flushing asynchronously.

#![cfg_attr(docsrs, feature(doc_cfg))]

use logforth_core::Error;
use logforth_core::kv;
use logforth_core::record::RecordOwned;

mod append;
mod state;
mod worker;

pub use self::append::Async;
pub use self::append::AsyncBuilder;

enum Task {
    Log {
        record: Box<RecordOwned>,
        diags: Vec<(kv::KeyOwned, kv::ValueOwned)>,
    },
    Flush {
        done: oneshot::Sender<Option<Error>>,
    },
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
enum Overflow {
    /// Blocks until the channel is not full.
    Block,
    /// Drops the incoming operation.
    DropIncoming,
}
