//! Wait-free thread-safe bounded channel.

use std::usize;
use std::num::Wrapping;
use std::sync::atomic::AtomicBool;

use util::{RingBuf, Counter, check_gte};

#[derive(Debug)]
pub struct Sender<T> {
    buf: RingBuf<Head, T>,
    sent_cache: Wrapping<usize>,
    received_cache: Wrapping<usize>,
}

#[derive(Debug)]
pub struct Receiver<T> {
    buf: RingBuf<Head, T>,
    sent_cache: Wrapping<usize>,
    received_cache: Wrapping<usize>,
}

#[derive(Debug)]
struct Head {
    sent: Counter,
    received: Counter,
    closed: AtomicBool,
}

#[derive(Debug)]
pub struct SendError<T> {
    pub kind: SendErrorKind,
    pub msg: T,
}

#[derive(Debug)]
pub enum SendErrorKind {
    BufferFull,
    ReceiverClosed,
}

#[derive(Debug)]
pub struct ReceiveError<T> {
    pub msg: T,
}

/// Create SPSC channel with given capacity.
///
/// # Panics
///
/// It panics if given capacity doesn't meet limitations below.
///
/// - Capacity should be power of 2
/// - Capacity should not have its MSB as 1, as it's used for overflow detection
///
pub fn channel<T: Send>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    use std::isize;
    assert!(capacity.is_power_of_two() && capacity & (isize::MAX as usize) == 0);

    let head = Head {
        sent: Counter::new(),
        received: Counter::new(),
        closed: AtomicBool::new(false),
    };

    let buf = RingBuf::new(head, capacity);

    let sender = Sender {
        buf: buf.clone(),
        sent_cache: Wrapping(0),
        received_cache: Wrapping(0),
    };
    let receiver = Receiver {
        buf,
        sent_cache: Wrapping(0),
        received_cache: Wrapping(0),
    };

    (sender, receiver)
}

impl<T: Send> Sender<T> {
    pub fn try_send(&mut self, msg: T) -> Result<(), SendError<T>> {
        if self.buffer_hint() == 0 {
            self.update_received();
        }

        if self.buffer_hint() == 0 {
            // buffer is full
            Err(SendError {
                kind: SendErrorKind::BufferFull,
                msg,
            })
        } else {
            unsafe {
                self.buf.set(self.sent_cache.0, msg);
            }

            self.increase_sent();

            Ok(())
        }
    }

    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    pub fn is_closed(&self) -> bool {
        // self.closed_cache || self.buf.head().closed.
        unimplemented!()
    }

    pub fn buffer_hint(&self) -> usize {
        let (sent, received) = check_gte(self.sent_cache, self.received_cache);

        debug_assert!(sent <= received + self.capacity());

        self.capacity() + received - sent
    }

    fn update_received(&mut self) {
        let received = Wrapping(self.buf.head().received.get());

        if cfg!(debug_assertions) {
            let (origin, cached) = check_gte(received, self.received_cache);

            assert!(origin >= cached);
        }

        self.received_cache = received;
    }

    fn increase_sent(&mut self) {
        let prev_sent = Wrapping(self.buf.head().sent.incr());
        debug_assert_eq!(prev_sent, self.sent_cache);
        self.sent_cache = prev_sent + Wrapping(1);
    }
}

impl<T: Send> Receiver<T> {
    pub fn try_recv(&mut self) -> Result<T, ()> {
        if self.buffer_hint() == 0 {
            self.update_sent();
        }

        if self.buffer_hint() == 0 {
            // buffer is empty
            Err(())
        } else {
            let msg = unsafe {
                self.buf.take(self.received_cache.0)
            };

            self.increase_received();

            Ok(msg)
        }
    }

    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    pub fn buffer_hint(&self) -> usize {
        let (sent, received) = check_gte(self.sent_cache, self.received_cache);

        debug_assert!(received <= sent);

        sent - received
    }

    fn update_sent(&mut self) {
        let sent = Wrapping(self.buf.head().sent.get());

        if cfg!(debug_assertions) {
            let (origin, cached) = check_gte(sent, self.sent_cache);

            assert!(origin >= cached);
        }

        self.sent_cache = sent;
    }

    fn increase_received(&mut self) {
        let prev_received = Wrapping(self.buf.head().received.incr());
        debug_assert_eq!(prev_received, self.sent_cache);
        self.received_cache = prev_received + Wrapping(1);
    }
}
