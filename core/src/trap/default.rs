use std::io;
use std::io::Write;

use crate::Error;
use crate::trap::Trap;

/// A default trap that sends errors to standard error if possible.
///
/// If standard error is not available, it does nothing.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct DefaultTrap {}

impl Trap for DefaultTrap {
    fn trap(&self, err: &Error) {
        let _ = writeln!(io::stderr(), "{err}");
    }
}
