use std::sync::atomic::Ordering;
use std::cell::Cell;

use sequence::{Sequence, Limit, Shared};
use counter::Counter;
use ringbuf::RingBuf;

use super::Head;

/// The transmission end of the queue.
///
/// This value is created by the [`queue`](queue) function.
#[derive(Debug)]
pub struct Sender<S: Sequence, R: Sequence, E: Default, T: Send> {
    buf: RingBuf<Head<S, R, E>, T>,
    capacity: usize,
    cache: S::Cache,
    is_closed_cache: Cell<bool>,
}

/// Error that emitted when sending failed.
#[derive(Debug)]
pub struct SendError<T: Send> {
    /// Reason why it failed.
    pub kind: SendErrorKind,

    /// Message attempted to be sent.
    pub payload: T,
}

/// Indicates why sending failed.
#[derive(Debug, Clone, Copy)]
pub enum SendErrorKind {
    /// Backing buffer is full.
    BufferFull,

    /// Every receivers are dropped, messages will be lost if sent.
    ReceiverAllClosed,
}

#[derive(Debug)]
struct UnusedLimit<'a, S: Sequence + 'a>(usize, &'a S);

impl<'a, S: Sequence> Limit for UnusedLimit<'a, S> {
    fn count(&self) -> Counter {
        self.1.count() + self.0
    }
}

impl<S: Sequence, R: Sequence, E: Default, T: Send> Sender<S, R, E, T> {
    pub(super) fn new(
        buf: RingBuf<Head<S, R, E>, T>, cache: S::Cache
    ) -> Self {
        Sender {
            capacity: buf.capacity(),
            buf,
            cache,
            is_closed_cache: false.into(),
        }
    }

    /// Total capacity of backing buffer of this channel.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Check if this channel is closed.
    pub fn is_closed(&self) -> bool {
        if self.is_closed_cache.get() {
            return true;
        }

        let head = self.buf.head();
        let is_closed = head.is_closed.load(Ordering::Acquire);

        if is_closed {
            self.is_closed_cache.set(true);
            true
        } else {
            false
        }
    }

    /// Try to send a message if possible.
    pub fn try_send(&mut self, msg: T) -> Result<(), SendError<T>> {
        if self.is_closed() {
            return Err(SendError {
                kind: SendErrorKind::ReceiverAllClosed,
                payload: msg,
            });
        }

        let head = self.buf.head();

        match head.sender.claim(&mut self.cache, UnusedLimit(self.capacity, &head.receiver)) {
            None => Err(SendError {
                kind: SendErrorKind::BufferFull,
                payload: msg,
            }),
            Some(index) => {
                unsafe {
                    self.buf.write(index, msg);
                }
                head.sender.commit(&mut self.cache, index);
                Ok(())
            }
        }
    }

    /// Expose extension part
    pub fn ext(&self) -> &E {
        &self.buf.head().extension
    }
}

impl<S, R, E, T> Drop for Sender<S, R, E, T> where S: Sequence, R: Sequence, E: Default, T: Send {
    fn drop(&mut self) {
        let head = self.buf.head();
        let remain_count = head.senders_count.fetch_sub(1, Ordering::Release);

        if remain_count == 1 {
            // This was the last sender
            head.is_closed.store(true, Ordering::Release);
        }
    }
}

impl<R, E, T> Clone for Sender<Shared, R, E, T> where
    R: Sequence,
    E: Default,
    T: Send,
{
    fn clone(&self) -> Self {
        self.buf.head().senders_count.fetch_add(1, Ordering::Relaxed);

        Sender {
            buf: self.buf.clone(),
            capacity: self.capacity,
            cache: self.cache,
            is_closed_cache: self.is_closed().into(),
        }
    }
}
