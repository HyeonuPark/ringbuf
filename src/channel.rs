//! Thread safe channel implementation using [`RingBuf`](../ringbuf/struct.RingBuf.html)
//! and [`Sequence`](../sequence/trait.Sequence.html).
//!
//! This module is fully generalized over spsc, spmc, mpsc, and mpmc.
//! Each specialized variants are also provided in their module for convenience.

use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering as O};
use std::ops::Drop;

use ringbuf::RingBuf;
use sequence::{Sequence, Shared};

#[derive(Debug)]
struct Head<S: Sequence, R: Sequence> {
    sender: S,
    receiver: R,
    is_closed: AtomicBool,
    sender_count: AtomicUsize,
    receiver_count: AtomicUsize,
}

/// The transmission end of the channel.
///
/// This value is created by the [`channel`](channel) function.
#[derive(Debug)]
pub struct Sender<S: Sequence, R: Sequence, T: Send> {
    buf: RingBuf<Head<S, R>, T>,
    cache: S::Cache,
}

/// The receiving end of the channel.
///
/// This value is created by the [`channel`](channel) function.
#[derive(Debug)]
pub struct Receiver<S: Sequence, R: Sequence, T: Send> {
    buf: RingBuf<Head<S, R>, T>,
    cache: R::Cache,
}

/// Error that emitted when sending failed.
#[derive(Debug)]
pub struct SendError<T: Send> {
    /// Reason why it failed.
    pub kind: SendErrorKind,

    /// Message attempted to sent.
    pub payload: T,
}

/// Indicates why sending failed.
#[derive(Debug, Clone, Copy)]
pub enum SendErrorKind {
    /// Backing buffer is full.
    BufferFull,

    /// Every receivers are dropped, messages are lost if sent.
    ReceiverAllClosed,
}

/// Error that emitted when receiving failed.
///
/// Receive can fail when buffer is empty.
/// Note that receive returns `Ok(None)` when every corresponding senders are dropped.
#[derive(Debug)]
pub struct ReceiveError;

/// Creates a bounded channel for communicating between asynchronous tasks.
pub fn channel<S, R, T>(capacity: usize) -> (Sender<S, R, T>, Receiver<S, R, T>) where
    S: Sequence,
    R: Sequence,
    T: Send
{
    let (sender, receiver, sender_cache, receiver_cache) = Default::default();

    let head = Head {
        sender,
        receiver,
        is_closed: AtomicBool::new(false),
        sender_count: AtomicUsize::new(1),
        receiver_count: AtomicUsize::new(1),
    };

    let buf = RingBuf::new(head, capacity);

    let sender = Sender {
        buf: buf.clone(),
        cache: sender_cache,
    };
    let receiver = Receiver {
        buf,
        cache: receiver_cache,
    };

    (sender, receiver)
}

impl<S: Sequence, R: Sequence, T: Send> Sender<S, R, T> {
    /// Total capacity of backing buffer of this channel.
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Tries to send a message if possible.
    pub fn try_send(&mut self, msg: T) -> Result<(), SendError<T>> {
        let head = self.buf.head();

        if head.is_closed.load(O::Relaxed) {
            return Err(SendError {
                kind: SendErrorKind::ReceiverAllClosed,
                payload: msg,
            });
        }

        match head.sender.claim(&mut self.cache, &head.receiver) {
            None => Err(SendError {
                kind: SendErrorKind::BufferFull,
                payload: msg,
            }),
            Some(index) => {
                unsafe {
                    self.buf.set(index, msg);
                }
                head.sender.commit(&mut self.cache, index);
                Ok(())
            }
        }
    }
}

impl<S: Sequence, R: Sequence, T: Send> Drop for Sender<S, R, T> {
    fn drop(&mut self) {
        self.buf.head().sender_count.fetch_sub(1, O::Relaxed);
    }
}

impl<R: Sequence, T: Send> Clone for Sender<Shared, R, T> {
    fn clone(&self) -> Self {
        self.buf.head().sender_count.fetch_add(1, O::Relaxed);

        Sender {
            buf: self.buf.clone(),
            cache: self.cache,
        }
    }
}

impl<S: Sequence, R: Sequence, T: Send> Receiver<S, R, T> {
    /// Total capacity of backing buffer of this channel.
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Tries to receive a message if possible.
    pub fn try_recv(&mut self) -> Result<Option<T>, ReceiveError> {
        let head = self.buf.head();

        if head.sender_count.load(O::Relaxed) == 0 {
            return Ok(None);
        }

        match head.receiver.claim(&mut self.cache, &head.sender) {
            None => Err(ReceiveError),
            Some(index) => {
                let msg = unsafe {
                    self.buf.take(index)
                };
                head.receiver.commit(&mut self.cache, index);
                Ok(Some(msg))
            }
        }
    }

    /// Close channel to prevent to send more messages.
    pub fn close(&mut self) {
        self.buf.head().is_closed.store(true, O::Relaxed);
    }
}

impl<S: Sequence, R: Sequence, T: Send> Drop for Receiver<S, R, T> {
    fn drop(&mut self) {
        let remain_count = self.buf.head().receiver_count.fetch_sub(1, O::Relaxed);

        if remain_count == 1 {
            // This was the last receiver
            self.buf.head().is_closed.store(false, O::Relaxed);
        }
    }
}

impl<S: Sequence, T: Send> Clone for Receiver<S, Shared, T> {
    fn clone(&self) -> Self {
        self.buf.head().receiver_count.fetch_add(1, O::Relaxed);

        Receiver {
            buf: self.buf.clone(),
            cache: self.cache,
        }
    }
}
