//! Thread safe generalized queue implementation
//! using [`RingBuf`](../ringbuf/struct.RingBuf.html)
//! and [`Sequence`](../sequence/trait.Sequence.html).

use ringbuf::{RingBuf, BufInfo};
use sequence::{Sequence, Limit, Shared};
use counter::Counter;

#[derive(Debug)]
struct Head<S: Sequence, R: Sequence> {
    sender: S,
    receiver: R,
}

/// The transmission end of the queue.
#[derive(Debug)]
pub struct Sender<S: Sequence, R: Sequence, T: Send> {
    buf: RingBuf<Head<S, R>, T>,
    cache: S::Cache,
}

/// Error that emitted when sending failed.
#[derive(Debug)]
pub struct SendError<T: Send>(pub T);

/// The receiving end of the queue.
#[derive(Debug)]
pub struct Receiver<S: Sequence, R: Sequence, T: Send> {
    buf: RingBuf<Head<S, R>, T>,
    cache: R::Cache,
}

/// Creates a bounded queue for communicating between asynchronous tasks.
pub fn queue<S, R, T>(
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
        Sender {
            buf: buf.clone(),
            cache: sender_cache,
        },
        Receiver {
            buf,
            cache: receiver_cache,
        },
    )
}

impl<S: Sequence, R: Sequence> BufInfo for Head<S, R> {
    fn start(&self) -> Counter {
        self.receiver.count()
    }

    fn end(&self) -> Counter {
        self.sender.count()
    }
}

#[derive(Debug)]
struct UnusedLimit<'a, S: Sequence + 'a>(usize, &'a S);

impl<'a, S: Sequence> Limit for UnusedLimit<'a, S> {
    fn count(&self) -> Counter {
        self.1.count() + self.0
    }
}

impl<S: Sequence, R: Sequence, T: Send> Sender<S, R, T> {
    /// Total capacity of backing buffer of this channel.
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Try to send a message if possible.
    pub fn try_send(&mut self, msg: T) -> Result<(), SendError<T>> {
        let head = self.buf.head();
        let capacity = self.capacity();

        match head.sender.claim(&mut self.cache, UnusedLimit(capacity, &head.receiver)) {
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
            cache: self.cache,
        }
    }
}

#[derive(Debug)]
struct SentLimit<'a, S: Sequence + 'a>(&'a S);

impl<'a, S: Sequence> Limit for SentLimit<'a, S> {
    fn count(&self) -> Counter {
        self.0.count()
    }
}

impl<S: Sequence, R: Sequence, T: Send> Receiver<S, R, T> {
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
}

impl<S, T> Clone for Receiver<S, Shared, T> where
    S: Sequence,
    T: Send,
{
    fn clone(&self) -> Self {
        Receiver {
            buf: self.buf.clone(),
            cache: self.cache,
        }
    }
}
