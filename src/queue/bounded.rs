//! Thread safe generalized bounded queue implementation
//! using [`RingBuf`](../../ringbuf/struct.RingBuf.html)
//! and [`Sequence`](../../sequence/trait.Sequence.html).

use ringbuf::{RingBuf, BufInfo};
use sequence::{Sequence, Limit, Shared};
use counter::Counter;

#[derive(Debug)]
pub(super) struct Head<S: Sequence, R: Sequence> {
    sender: S,
    receiver: R,
}

/// The transmission end of the bounded queue.
#[derive(Debug)]
pub struct Sender<S: Sequence, R: Sequence, T: Send> {
    buf: RingBuf<Head<S, R>, T>,
    cache: S::Cache,
}

/// Error that emitted when sending failed.
#[derive(Debug)]
pub struct SendError<T: Send> {
    /// Reason why sending failed.
    pub kind: SendErrorKind,

    /// Message tried to be sent.
    pub msg: T,
}

/// Reason why sending failed.
#[derive(Debug)]
pub enum SendErrorKind {
    /// Buffer is full.
    BufferFull,

    /// This queue is closed.
    Closed,
}

#[derive(Debug)]
struct SenderLimit<'a, S: Sequence + 'a>(usize, &'a S);

/// The receiving end of the bounded queue.
#[derive(Debug)]
pub struct Receiver<S: Sequence, R: Sequence, T: Send> {
    buf: RingBuf<Head<S, R>, T>,
    cache: R::Cache,
}

#[derive(Debug)]
struct ReceiverLimit<'a, S: Sequence + 'a>(&'a S);

pub(super) fn queue_buf<S: Sequence, R: Sequence, T: Send>(
    capacity: usize
) -> RingBuf<Head<S, R>, T> {
    let (sender, receiver): (S, R) = Default::default();

    let head = Head {
        sender,
        receiver,
    };

    RingBuf::new(head, capacity)
}

/// Creates a bounded queue.
pub fn queue<S, R, T>(
    capacity: usize
) -> (Sender<S, R, T>, Receiver<S, R, T>) where
    S: Sequence,
    R: Sequence,
    T: Send,
{
    let buf = queue_buf(capacity);

    let sender = Sender::new(buf.clone());
    let receiver = Receiver::new(buf);

    (sender, receiver)
}

impl<S: Sequence, R: Sequence> BufInfo for Head<S, R> {
    fn start(&self) -> Counter {
        self.receiver.count()
    }

    fn end(&self) -> Counter {
        self.sender.count()
    }
}

impl<'a, S: Sequence> Limit for SenderLimit<'a, S> {
    fn count(&self) -> Counter {
        self.1.count() + self.0
    }
}

impl<S: Sequence, R: Sequence, T: Send> Sender<S, R, T> {
    pub(super) fn new(buf: RingBuf<Head<S, R>, T>) -> Self {
        let cache = buf.head().sender.cache(SenderLimit(buf.capacity(), &buf.head().receiver));

        Sender {
            buf,
            cache,
        }
    }

    /// Total capacity of backing buffer of this queue.
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Check if this queue is closed
    pub fn is_closed(&self) -> bool {
        self.buf.is_closed()
    }

    /// Try to send a message if possible.
    pub fn try_send(&mut self, msg: T) -> Result<(), SendError<T>> {
        let head = self.buf.head();
        let capacity = self.capacity();

        if self.is_closed() {
            return Err(SendError {
                kind: SendErrorKind::Closed,
                msg,
            });
        }

        match head.sender.claim(&mut self.cache, SenderLimit(capacity, &head.receiver)) {
            None => Err(SendError {
                kind: SendErrorKind::BufferFull,
                msg,
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

impl<'a, S: Sequence> Limit for ReceiverLimit<'a, S> {
    fn count(&self) -> Counter {
        self.0.count()
    }
}

impl<S: Sequence, R: Sequence, T: Send> Receiver<S, R, T> {
    pub(super) fn new(buf: RingBuf<Head<S, R>, T>) -> Self {
        let cache = buf.head().receiver.cache(ReceiverLimit(&buf.head().sender));

        Receiver {
            buf,
            cache,
        }
    }

    /// Total capacity of backing buffer of this queue.
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Check if this queue is closed.
    pub fn is_closed(&self) -> bool {
        self.buf.is_closed()
    }

    /// Close this queue.
    pub fn close(&mut self) {
        self.buf.close();
    }

    /// Try to receive a message if possible.
    pub fn try_recv(&mut self) -> Option<T> {
        let head = self.buf.head();

        match head.receiver.claim(&mut self.cache, ReceiverLimit(&head.sender)) {
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
