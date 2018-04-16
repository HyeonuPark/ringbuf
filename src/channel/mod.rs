//! Thread safe channel implementation using [`RingBuf`](../ringbuf/struct.RingBuf.html)
//! and [`Sequence`](../sequence/trait.Sequence.html).
//!
//! This module is fully generalized over spsc, spmc, mpsc, and mpmc.
//! Each specialized variants are also provided in their module for convenience.

use std::sync::atomic::{AtomicUsize, AtomicBool};
use ringbuf::RingBuf;
use sequence::Sequence;
use extension::Extension;

mod sender;
mod receiver;

pub use self::sender::{Sender, SendError, SendErrorKind};
pub use self::receiver::{Receiver, ReceiveError};

#[derive(Debug)]
pub(crate) struct Head<S: Sequence, R: Sequence, E: Extension> {
    sender: S,
    receiver: R,
    is_closed: AtomicBool,
    sender_count: AtomicUsize,
    receiver_count: AtomicUsize,
    pub(crate) ext: E,
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
    let (ext, ext_sender, ext_receiver) = E::create_triple();

    let head = Head {
        sender,
        receiver,
        is_closed: AtomicBool::new(false),
        sender_count: AtomicUsize::new(1),
        receiver_count: AtomicUsize::new(1),
        ext,
    };

    let buf = RingBuf::new(head, capacity);

    (
        Sender::new(buf.clone(), sender_cache, ext_sender),
        Receiver::new(buf, receiver_cache, ext_receiver),
    )
}
