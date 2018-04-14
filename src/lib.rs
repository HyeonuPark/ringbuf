//! Thread safe bounded channels based on ring buffer.
//!
//! This crate provides channels that can be used to communicate
//! between asynchronous tasks.
//!
//! Channels are based on fixed-sized ring buffer. Send operations simply fail
//! if backing buffer is full, and you can get back message with error.

#![deny(missing_docs)]

pub mod counter;
pub mod sequence;
pub mod ringbuf;
pub mod channel;

pub mod spsc {
    #![allow(missing_docs)]
    use sequence::Owned;
    use channel as chan;

    pub use channel::{SendError, SendErrorKind, ReceiveError};
    pub type Sender<T> = chan::Sender<Owned, Owned, T>;
    pub type Receiver<T> = chan::Receiver<Owned, Owned, T>;

    #[inline]
    pub fn channel<T: Send>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        chan::channel(capacity)
    }
}

pub mod mpsc {
    #![allow(missing_docs)]
    use sequence::{Owned, Shared};
    use channel as chan;

    pub use channel::{SendError, SendErrorKind, ReceiveError};
    pub type Sender<T> = chan::Sender<Shared, Owned, T>;
    pub type Receiver<T> = chan::Receiver<Shared, Owned, T>;

    #[inline]
    pub fn channel<T: Send>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        chan::channel(capacity)
    }
}

pub mod spmc {
    #![allow(missing_docs)]
    use sequence::{Owned, Shared};
    use channel as chan;

    pub use channel::{SendError, SendErrorKind, ReceiveError};
    pub type Sender<T> = chan::Sender<Owned, Shared, T>;
    pub type Receiver<T> = chan::Receiver<Owned, Shared, T>;

    #[inline]
    pub fn channel<T: Send>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        chan::channel(capacity)
    }
}

pub mod mpmc {
    #![allow(missing_docs)]
    use sequence::Shared;
    use channel as chan;

    pub use channel::{SendError, SendErrorKind, ReceiveError};
    pub type Sender<T> = chan::Sender<Shared, Shared, T>;
    pub type Receiver<T> = chan::Receiver<Shared, Shared, T>;

    #[inline]
    pub fn channel<T: Send>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        chan::channel(capacity)
    }
}
