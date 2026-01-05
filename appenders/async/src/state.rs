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

use std::sync::Arc;
use std::thread::JoinHandle;

use arc_swap::ArcSwapOption;
use logforth_core::Error;

use crate::Overflow;
use crate::Sender;
use crate::Task;

#[derive(Debug)]
pub(crate) struct AsyncState(ArcSwapOption<State>);

#[derive(Debug)]
struct State {
    overflow: Overflow,
    sender: Sender<Task>,
    handle: JoinHandle<()>,
}

impl AsyncState {
    pub(crate) fn new(overflow: Overflow, sender: Sender<Task>, handle: JoinHandle<()>) -> Self {
        Self(ArcSwapOption::from(Some(Arc::new(State {
            overflow,
            sender,
            handle,
        }))))
    }

    pub(crate) fn send_task(&self, task: Task) -> Result<(), Error> {
        let state = self.0.load();
        // SAFETY: state is always Some before dropped.
        let state = state.as_ref().unwrap();
        let sender = &state.sender;

        match state.overflow {
            Overflow::Block => sender.send(task).map_err(|err| {
                Error::new(match err.0 {
                    Task::Log { .. } => "failed to send log task to async appender",
                    Task::Flush { .. } => "failed to send flush task to async appender",
                })
            }),
            Overflow::DropIncoming => match sender.try_send(task) {
                Ok(()) => Ok(()),
                Err(std::sync::mpsc::TrySendError::Full(_)) => Ok(()),
                Err(std::sync::mpsc::TrySendError::Disconnected(task)) => {
                    Err(Error::new(match task {
                        Task::Log { .. } => "failed to send log task to async appender",
                        Task::Flush { .. } => "failed to send flush task to async appender",
                    }))
                }
            },
        }
    }

    pub(crate) fn destroy(&self) {
        if let Some(state) = self.0.swap(None) {
            // SAFETY: state has always one strong count before swapped.
            let State {
                overflow: _,
                sender,
                handle,
            } = Arc::into_inner(state).unwrap();

            // drop our sender, threads will break the loop after receiving and processing
            drop(sender);

            // wait for the thread to finish
            handle.join().expect("failed to join async appender thread");
        }
    }
}

impl Drop for AsyncState {
    fn drop(&mut self) {
        self.destroy();
    }
}
