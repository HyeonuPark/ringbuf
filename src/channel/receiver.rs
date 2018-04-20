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
}

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
        }
    }

    /// Total capacity of backing buffer of this channel.
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Try to receive a message if possible.
    pub fn try_recv(&mut self) -> Option<T> {
        let head = self.buf.head();

        match head.receiver.claim(&mut self.cache, SentLimit(&head.sender)) {
            None => None,
            Some(index) => {
                let msg = unsafe {
                    self.buf.read(index)
                };
                head.receiver.commit(&mut self.cache, index);
                Some(msg)
            }
        }
    }

    /// Expose extension part
    pub fn ext(&self) -> &E {
        &self.buf.head().extension
    }
}

impl<S, E, T> Clone for Receiver<S, Shared, E, T> where
    S: Sequence,
    E: Default,
    T: Send,
{
    fn clone(&self) -> Self {
        Receiver {
            buf: self.buf.clone(),
            capacity: self.capacity,
            cache: self.cache,
        }
    }
}
