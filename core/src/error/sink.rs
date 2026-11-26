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

use std::fmt;
use std::sync::Arc;

use crate::Error;

/// A terminal sink that processes internal runtime errors produced by the logging pipeline.
///
/// Logging operations are designed not to crash the host application at runtime. When an appender
/// fails to emit or flush records, execution roots such as [`Logger`](crate::Logger), an
/// overriding [`DispatchBuilder`](crate::DispatchBuilder), or asynchronous workers forward the
/// resulting [`Error`] to their configured [`ErrorSink`].
///
/// Because [`ErrorSink`] is the final stage of error handling, implementations operate on a
/// best-effort basis: they must silently absorb their own delivery failures, never panic, and
/// never route log records back into the owning [`Logger`](crate::Logger).
pub trait ErrorSink: fmt::Debug + Send + Sync + 'static {
    /// Process an internal runtime error on a best-effort basis.
    fn sink(&self, err: &Error);
}

impl<T: ErrorSink> From<T> for Box<dyn ErrorSink> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

impl<T: ErrorSink + ?Sized> ErrorSink for Arc<T> {
    fn sink(&self, err: &Error) {
        (**self).sink(err);
    }
}

impl<T: ErrorSink + ?Sized> ErrorSink for &'static T {
    fn sink(&self, err: &Error) {
        (**self).sink(err);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;

    use super::*;
    use crate::error::BlackHoleErrorSink;
    use crate::error::StderrErrorSink;

    #[derive(Debug)]
    struct CountingSink {
        count: AtomicUsize,
    }

    impl CountingSink {
        const fn new() -> Self {
            Self {
                count: AtomicUsize::new(0),
            }
        }
    }

    impl ErrorSink for CountingSink {
        fn sink(&self, _: &Error) {
            self.count.fetch_add(1, Ordering::Relaxed);
        }
    }

    static STATIC_SINK: CountingSink = CountingSink::new();

    #[test]
    fn builtin_sinks_are_zst_and_forward_wrappers_work() {
        assert_eq!(size_of::<StderrErrorSink>(), 0);
        assert_eq!(size_of::<BlackHoleErrorSink>(), 0);

        let black_hole: Box<dyn ErrorSink> = BlackHoleErrorSink::default().into();
        black_hole.sink(&Error::new("ignored"));

        let shared = Arc::new(CountingSink::new());
        let boxed_concrete: Box<dyn ErrorSink> = shared.clone().into();
        let dyn_arc: Arc<dyn ErrorSink> = shared.clone();
        let boxed_dyn: Box<dyn ErrorSink> = dyn_arc.into();

        boxed_concrete.sink(&Error::new("one"));
        boxed_dyn.sink(&Error::new("two"));
        assert_eq!(shared.count.load(Ordering::Relaxed), 2);

        let static_ref: Box<dyn ErrorSink> = (&STATIC_SINK).into();
        static_ref.sink(&Error::new("three"));
        assert_eq!(STATIC_SINK.count.load(Ordering::Relaxed), 1);
    }
}
