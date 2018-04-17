use std::sync::atomic::Ordering;
use std::cell::Cell;

use sequence::{Sequence, Limit, Shared};
use counter::Counter;
use ringbuf::RingBuf;
use extension::Extension;

use super::Head;

/// The receiving end of the queue.
///
/// This value is created by the [`queue`](queue) function.
#[derive(Debug)]
pub struct Receiver<S: Sequence, R: Sequence, E: Extension, T: Send> {
    buf: RingBuf<Head<S, R, E>, T>,
    capacity: usize,
    cache: R::Cache,
    extension: E::Receiver,
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

impl<S: Sequence, R: Sequence, E: Extension, T: Send> Receiver<S, R, E, T> {
    pub(super) fn new(
        buf: RingBuf<Head<S, R, E>, T>, cache: R::Cache, extension: E::Receiver
    ) -> Self {
        Receiver {
            capacity: buf.capacity(),
            buf,
            cache,
            extension,
            is_closed_cache: false.into(),
        }
    }

    /// Total capacity of backing buffer of this channel.
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Tries to receive a message if possible.
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
                    self.buf.take(index)
                };
                head.receiver.commit(&mut self.cache, index);
                Ok(Some(msg))
            }
        }
    }

    /// Close channel to prevent to send more messages.
    pub fn close(&mut self) {
        if !self.is_closed() {
            self.is_closed_cache.set(true);
            self.buf.head().is_closed.store(true, Ordering::Relaxed);
        }
    }

    /// Check if this channel is closed.
    pub fn is_closed(&self) -> bool {
        if self.is_closed_cache.get() {
            return true;
        }

        let head = self.buf.head();
        let is_closed = head.is_closed.load(Ordering::Relaxed) ||
                head.senders.load(Ordering::Relaxed) == 0;

        if is_closed {
            self.is_closed_cache.set(true);
            true
        } else {
            false
        }
    }

    /// Expose local part of extension
    pub fn ext(&self) -> &E::Receiver {
        &self.extension
    }

    /// Expose local part of extension mutably
    pub fn ext_mut(&mut self) -> &mut E::Receiver {
        &mut self.extension
    }

    /// Expose shared part of extension
    pub fn ext_head(&self) -> &E {
        &self.buf.head().extension
    }
}

impl<S: Sequence, R: Sequence, E: Extension, T: Send> Drop for Receiver<S, R, E, T> {
    fn drop(&mut self) {
        let remain_count = self.buf.head().receivers.fetch_sub(1, Ordering::Relaxed);

        if remain_count == 1 {
            // This was the last receiver
            self.buf.head().is_closed.store(false, Ordering::Relaxed);
        }
    }
}

impl<S, E, T> Clone for Receiver<S, Shared, E, T> where
    S: Sequence,
    E: Extension,
    E::Receiver: Clone,
    T: Send,
{
    fn clone(&self) -> Self {
        self.buf.head().receivers.fetch_add(1, Ordering::Relaxed);

        Receiver {
            buf: self.buf.clone(),
            capacity: self.capacity,
            cache: self.cache.clone(),
            extension: self.extension.clone(),
            is_closed_cache: self.is_closed().into(),
        }
    }
}
