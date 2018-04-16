use std::sync::atomic::Ordering;

use sequence::{Sequence, Limit, Shared};
use counter::Counter;
use ringbuf::RingBuf;
use extension::Extension;

use super::Head;

/// The transmission end of the channel.
///
/// This value is created by the [`channel`](channel) function.
#[derive(Debug)]
pub struct Sender<S: Sequence, R: Sequence, E: Extension, T: Send> {
    buf: RingBuf<Head<S, R, E>, T>,
    capacity: usize,
    cache: S::Cache,
    ext: E::Sender,
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

impl<S: Sequence, R: Sequence, E: Extension, T: Send> Sender<S, R, E, T> {
    pub(super) fn new(buf: RingBuf<Head<S, R, E>, T>, cache: S::Cache, ext: E::Sender) -> Self {
        Sender {
            capacity: buf.capacity(),
            buf,
            cache,
            ext,
        }
    }

    /// Total capacity of backing buffer of this channel.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Tries to send a message if possible.
    pub fn try_send(&mut self, msg: T) -> Result<(), SendError<T>> {
        let head = self.buf.head();

        if head.is_closed.load(Ordering::Relaxed) {
            return Err(SendError {
                kind: SendErrorKind::ReceiverAllClosed,
                payload: msg,
            });
        }

        match head.sender.claim(&mut self.cache, UnusedLimit(self.capacity, &head.receiver)) {
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

    /// Expose local part of extension
    pub fn ext(&self) -> &E::Sender {
        &self.ext
    }

    /// Expose local part of extension mutably
    pub fn ext_mut(&mut self) -> &mut E::Sender {
        &mut self.ext
    }

    /// Expose shared part of extension
    pub fn ext_head(&self) -> &E {
        &self.buf.head().ext
    }
}

impl<S: Sequence, R: Sequence, E: Extension, T: Send> Drop for Sender<S, R, E, T> {
    fn drop(&mut self) {
        let head = self.buf.head();
        head.sender_count.fetch_sub(1, Ordering::Relaxed);
        head.ext.cleanup_sender(&mut self.ext);
    }
}

impl<R, E, T> Clone for Sender<Shared, R, E, T> where
    R: Sequence,
    E: Extension,
    E::Sender: Clone,
    T: Send,
{
    fn clone(&self) -> Self {
        self.buf.head().sender_count.fetch_add(1, Ordering::Relaxed);

        Sender {
            buf: self.buf.clone(),
            capacity: self.capacity,
            cache: self.cache.clone(),
            ext: self.ext.clone(),
        }
    }
}
