//! Thread safe generalized queue implementation
//! using [`RingBuf`](../ringbuf/struct.RingBuf.html)
//! and [`Sequence`](../sequence/trait.Sequence.html).

use std::sync::atomic::{AtomicUsize, AtomicBool};
use ringbuf::RingBuf;
use sequence::Sequence;
use extension::Extension;

mod sender;
mod receiver;

pub use self::sender::{Sender, SendError, SendErrorKind};
pub use self::receiver::{Receiver, ReceiveError};

#[derive(Debug)]
struct Head<S: Sequence, R: Sequence, E: Extension> {
    sender: S,
    receiver: R,
    is_closed: AtomicBool,
    senders: AtomicUsize,
    receivers: AtomicUsize,
    extension: E,
}

/// Creates a bounded channel for communicating between asynchronous tasks.
pub fn channel<S, R, E, T>(
    capacity: usize
) -> (Sender<S, R, E, T>, Receiver<S, R, E, T>) where
    S: Sequence,
    R: Sequence,
    E: Extension,
    T: Send,
{
    let (sender, receiver, sender_cache, receiver_cache) = Default::default();
    let (extension, ext_sender, ext_receiver) = E::create_triple();

    let head = Head {
        sender,
        receiver,
        is_closed: AtomicBool::new(false),
        senders: AtomicUsize::new(1),
        receivers: AtomicUsize::new(1),
        extension,
    };

    let buf = RingBuf::new(head, capacity);

    (
        Sender::new(buf.clone(), sender_cache, ext_sender),
        Receiver::new(buf, receiver_cache, ext_receiver),
    )
}
