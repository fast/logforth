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

use std::io;
use std::io::Write;

use crossbeam_channel::Receiver;
use crossbeam_channel::RecvError;
use crossbeam_channel::TryRecvError;

use super::Message;

pub(crate) struct Worker<T: Write + Send + 'static> {
    writer: T,
    receiver: Receiver<Message>,
    shutdown: Receiver<()>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum WorkerState {
    Empty,
    Disconnected,
    Continue,
    Shutdown,
}

impl<T: Write + Send + 'static> Worker<T> {
    pub(crate) fn new(writer: T, receiver: Receiver<Message>, shutdown: Receiver<()>) -> Worker<T> {
        Self {
            writer,
            receiver,
            shutdown,
        }
    }

    fn recv(&mut self) -> io::Result<WorkerState> {
        match self.receiver.recv() {
            Ok(Message::Record(record)) => {
                self.writer.write_all(&record)?;
                Ok(WorkerState::Continue)
            }
            Ok(Message::Shutdown) => Ok(WorkerState::Shutdown),
            Err(RecvError) => Ok(WorkerState::Disconnected),
        }
    }

    fn try_recv(&mut self) -> io::Result<WorkerState> {
        match self.receiver.try_recv() {
            Ok(Message::Record(record)) => {
                self.writer.write_all(&record)?;
                Ok(WorkerState::Continue)
            }
            Ok(Message::Shutdown) => Ok(WorkerState::Shutdown),
            Err(TryRecvError::Empty) => Ok(WorkerState::Empty),
            Err(TryRecvError::Disconnected) => Ok(WorkerState::Disconnected),
        }
    }

    pub(crate) fn work(&mut self) -> io::Result<WorkerState> {
        let mut worker_state = self.recv()?;

        while worker_state == WorkerState::Continue {
            worker_state = self.try_recv()?;
        }

        self.writer.flush()?;
        Ok(worker_state)
    }

    pub(crate) fn make_thread(mut self, name: String) -> std::thread::JoinHandle<()> {
        std::thread::Builder::new()
            .name(name)
            .spawn(move || {
                loop {
                    match self.work() {
                        Ok(WorkerState::Continue) | Ok(WorkerState::Empty) => {}
                        Ok(WorkerState::Shutdown) | Ok(WorkerState::Disconnected) => {
                            let _ = self.shutdown.recv();
                            break;
                        }
                        Err(err) => {
                            eprintln!("failed to write log: {err}");
                        }
                    }
                }
                if let Err(err) = self.writer.flush() {
                    eprintln!("failed to flush: {err}");
                }
            })
            .expect("failed to spawn the non-blocking rolling file writer thread")
    }
}
