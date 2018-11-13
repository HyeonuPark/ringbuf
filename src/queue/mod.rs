
pub(crate) mod head;
pub(crate) mod half;
pub(crate) mod bounded;

pub use self::bounded::{Sender, Receiver, SendError, RecvError};
pub use self::bounded::queue as create;

#[cfg(test)]
mod tests;
