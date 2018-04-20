use std::sync::atomic::Ordering;
use std::cell::Cell;

use sequence::{Sequence, Limit, Shared};
use counter::Counter;
use ringbuf::RingBuf;

use super::Head;

/// The receiving end of the queue.
///
/// This value is created by the [`queue`](queue) function.
#[derive(Debug)]
pub struct Receiver<S: Sequence, R: Sequence, E: Default, T: Send> {
    buf: RingBuf<Head<S, R, E>, T>,
    capacity: usize,
    cache: R::Cache,
    is_closed_cache: Cell<bool>,
}

/// Error which can be emitted when receiving failed.
///
/// Receive can fail when buffer is empty.
/// Note that receive returns `Ok(None)` when every corresponding senders are dropped.
#[derive(Debug)]
pub struct ReceiveError;

#[derive(Debug)]
struct SentLimit<'a, S: Sequence + 'a>(&'a S);

impl<'a, S: Sequence> Limit for SentLimit<'a, S> {
    fn count(&self) -> Counter {
        self.0.count()
    }
}

impl<S: Sequence, R: Sequence, E: Default, T: Send> Receiver<S, R, E, T> {
    pub(super) fn new(
        buf: RingBuf<Head<S, R, E>, T>, cache: R::Cache
    ) -> Self {
        Receiver {
            capacity: buf.capacity(),
            buf,
            cache,
            is_closed_cache: false.into(),
        }
    }

    /// Total capacity of backing buffer of this channel.
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Close channel to prevent to send more messages.
    pub fn close(&mut self) {
        if !self.is_closed() {
            self.is_closed_cache.set(true);
            self.buf.head().is_closed.store(true, Ordering::Release);
        }
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

    /// Try to receive a message if possible.
    pub fn try_recv(&mut self) -> Result<Option<T>, ReceiveError> {
        let head = self.buf.head();

        match head.receiver.claim(&mut self.cache, SentLimit(&head.sender)) {
            None => {
                if self.is_closed() {
                    Ok(None)
                } else {
                    Err(ReceiveError)
                }
            }
            Some(index) => {
                let msg = unsafe {
                    self.buf.read(index)
                };
                head.receiver.commit(&mut self.cache, index);
                Ok(Some(msg))
            }
        }
    }

    /// Expose extension part
    pub fn ext(&self) -> &E {
        &self.buf.head().extension
    }
}

impl<S: Sequence, R: Sequence, E: Default, T: Send> Drop for Receiver<S, R, E, T> {
    fn drop(&mut self) {
        let head = self.buf.head();
        let remain_count = head.receivers_count.fetch_sub(1, Ordering::Release);

        if remain_count == 1 {
            // This was the last receiver
            head.is_closed.store(true, Ordering::Release);
        }
    }
}

impl<S, E, T> Clone for Receiver<S, Shared, E, T> where
    S: Sequence,
    E: Default,
    T: Send,
{
    fn clone(&self) -> Self {
        self.buf.head().receivers_count.fetch_add(1, Ordering::Relaxed);

        Receiver {
            buf: self.buf.clone(),
            capacity: self.capacity,
            cache: self.cache,
            is_closed_cache: self.is_closed().into(),
        }
    }
}
