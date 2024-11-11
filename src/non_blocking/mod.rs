mod non_blocking;
mod worker;

pub use non_blocking::NonBlocking;
pub use non_blocking::NonBlockingBuilder;
pub use non_blocking::WorkerGuard;

#[derive(Debug)]
enum Message {
    Record(Vec<u8>),
    Shutdown,
}
