

mod append;
mod non_blocking;
mod rolling;
mod worker;


#[derive(Debug)]
enum Message {
    Record(Vec<u8>),
    Shutdown,
}
