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

use std::sync::mpsc;

pub(crate) fn channel<T>(bound: Option<usize>) -> (Sender<T>, Receiver<T>) {
    match bound {
        Some(bound) => {
            let (tx, rx) = mpsc::sync_channel(bound);
            (Sender::Bounded(tx), rx)
        }
        None => {
            let (tx, rx) = mpsc::channel();
            (Sender::Unbounded(tx), rx)
        }
    }
}

pub(crate) type Receiver<T> = mpsc::Receiver<T>;

#[derive(Clone)]
pub(crate) enum Sender<T> {
    Unbounded(mpsc::Sender<T>),
    Bounded(mpsc::SyncSender<T>),
}

impl<T> std::fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sender::Unbounded(tx) => tx.fmt(f),
            Sender::Bounded(tx) => tx.fmt(f),
        }
    }
}

impl<T> Sender<T> {
    pub(crate) fn send(&self, value: T) -> Result<(), mpsc::SendError<T>> {
        match self {
            Sender::Unbounded(s) => s.send(value),
            Sender::Bounded(s) => s.send(value),
        }
    }

    pub(crate) fn try_send(&self, value: T) -> Result<(), mpsc::TrySendError<T>> {
        match self {
            Sender::Unbounded(s) => s
                .send(value)
                .map_err(|e| mpsc::TrySendError::Disconnected(e.0)),
            Sender::Bounded(s) => s.try_send(value),
        }
    }
}
