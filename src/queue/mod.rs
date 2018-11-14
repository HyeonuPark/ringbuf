
pub mod bounded;
pub mod unordered;

pub use self::bounded::{queue, Sender, Receiver, SendError, RecvError};

#[cfg(test)]
mod tests;
