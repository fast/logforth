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

use log::SetLoggerError;

#[derive(Debug, thiserror::Error)]
pub enum LogSetupError {
    #[error("failed to perform IO action: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to set up logger: {0}")]
    SetLogger(SetLoggerError),
}

impl From<SetLoggerError> for LogSetupError {
    fn from(value: SetLoggerError) -> Self {
        LogSetupError::SetLogger(value)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LogError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
}
