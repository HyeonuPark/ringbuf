use sequence::{Sequence, Limit, Shared};
use counter::Counter;
use ringbuf::RingBuf;

use super::Head;

/// The transmission end of the queue.
///
/// This value is created by the [`queue`](queue) function.
#[derive(Debug)]
pub struct Sender<S: Sequence, R: Sequence, T: Send> {
    buf: RingBuf<Head<S, R>, T>,
    capacity: usize,
    cache: S::Cache,
}

/// Error that emitted when sending failed.
#[derive(Debug)]
pub struct SendError<T: Send>(pub T);

#[derive(Debug)]
struct UnusedLimit<'a, S: Sequence + 'a>(usize, &'a S);

impl<'a, S: Sequence> Limit for UnusedLimit<'a, S> {
    fn count(&self) -> Counter {
        self.1.count() + self.0
    }
}

impl<S: Sequence, R: Sequence, T: Send> Sender<S, R, T> {
    pub(super) fn new(
        buf: RingBuf<Head<S, R>, T>, cache: S::Cache
    ) -> Self {
        Sender {
            capacity: buf.capacity(),
            buf,
            cache,
        }
    }

    /// Total capacity of backing buffer of this channel.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Try to send a message if possible.
    pub fn try_send(&mut self, msg: T) -> Result<(), SendError<T>> {
        let head = self.buf.head();

        match head.sender.claim(&mut self.cache, UnusedLimit(self.capacity, &head.receiver)) {
            None => Err(SendError(msg)),
            Some(index) => {
                unsafe {
                    self.buf.write(index, msg);
                }
                head.sender.commit(&mut self.cache, index);
                Ok(())
            }
        }
    }
}

impl<R, T> Clone for Sender<Shared, R, T> where
    R: Sequence,
    T: Send,
{
    fn clone(&self) -> Self {
        Sender {
            buf: self.buf.clone(),
            capacity: self.capacity,
            cache: self.cache,
        }
    }
}
