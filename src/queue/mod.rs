//! Thread safe generalized queue implementation
//! using [`RingBuf`](../ringbuf/struct.RingBuf.html)
//! and [`Sequence`](../sequence/trait.Sequence.html).

use ringbuf::{RingBuf, BufInfo};
use sequence::Sequence;
use counter::Counter;

mod sender;
mod receiver;

pub use self::sender::{Sender, SendError};
pub use self::receiver::Receiver;

#[derive(Debug)]
struct Head<S: Sequence, R: Sequence> {
    sender: S,
    receiver: R,
}

impl<S: Sequence, R: Sequence> BufInfo for Head<S, R> {
    fn start(&self) -> Counter {
        self.receiver.count()
    }

    fn end(&self) -> Counter {
        self.sender.count()
    }
}

/// Creates a bounded channel for communicating between asynchronous tasks.
pub fn channel<S, R, T>(
    capacity: usize
) -> (Sender<S, R, T>, Receiver<S, R, T>) where
    S: Sequence,
    R: Sequence,
    T: Send,
{
    let (sender, receiver, sender_cache, receiver_cache) = Default::default();

    let head = Head {
        sender,
        receiver,
    };

    let buf = RingBuf::new(head, capacity);

    (
        Sender::new(buf.clone(), sender_cache),
        Receiver::new(buf, receiver_cache),
    )
}

#[cfg(test)]
mod tests;
