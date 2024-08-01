// Copyright 2024 tison <wander4096@gmail.com>
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

pub use append::RollingFile;
pub use non_blocking::NonBlocking;
pub use non_blocking::NonBlockingBuilder;
pub use non_blocking::WorkerGuard;
pub use rolling::RollingFileWriter;
pub use rolling::RollingFileWriterBuilder;
pub use rolling::Rotation;

mod append;
mod non_blocking;
mod rolling;
mod worker;

#[derive(Debug)]
enum Message {
    Record(Vec<u8>),
    Shutdown,
}
