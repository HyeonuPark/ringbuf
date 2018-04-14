
use std::sync::atomic::{AtomicUsize, Ordering as O};
use std::ops::Drop;

use ringbuf::RingBuf;
use sequence::{Sequence, Shared};

#[derive(Debug)]
struct Head<S: Sequence, R: Sequence> {
    sender: S,
    receiver: R,
    sender_count: AtomicUsize,
    receiver_count: AtomicUsize,
}

#[derive(Debug)]
pub struct Sender<S: Sequence, R: Sequence, T: Send> {
    buf: RingBuf<Head<S, R>, T>,
    cache: S::Cache,
}

#[derive(Debug)]
pub struct Receiver<S: Sequence, R: Sequence, T: Send> {
    buf: RingBuf<Head<S, R>, T>,
    cache: R::Cache,
}

#[derive(Debug)]
pub struct SendError<T: Send> {
    pub kind: SendErrorKind,
    pub payload: T,
}

#[derive(Debug, Clone, Copy)]
pub enum SendErrorKind {
    BufferFull,
    ReceiverAllClosed,
}

#[derive(Debug)]
pub struct ReceiveError;

pub fn channel<S, R, T>(capacity: usize) -> (Sender<S, R, T>, Receiver<S, R, T>) where
    S: Sequence,
    R: Sequence,
    T: Send
{
    let (sender, receiver, sender_cache, receiver_cache) = Default::default();

    let head = Head {
        sender,
        receiver,
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
    pub fn try_send(&mut self, msg: T) -> Result<(), SendError<T>> {
        let head = self.buf.head();

        if head.receiver_count.load(O::Relaxed) == 0 {
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
}

impl<S: Sequence, R: Sequence, T: Send> Drop for Receiver<S, R, T> {
    fn drop(&mut self) {
        self.buf.head().receiver_count.fetch_sub(1, O::Relaxed);
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
