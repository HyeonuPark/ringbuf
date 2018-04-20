//! Thread safe bounded channels based on ring buffer.
//!
//! This crate provides channels that can be used to communicate
//! between asynchronous tasks.
//!
//! Channels are based on fixed-sized ring buffer. Send operations simply fail
//! if backing buffer is full, and you can get back message you sent from error.

#![deny(missing_docs)]

extern crate crossbeam;
extern crate futures;

#[cfg(test)]
extern crate rand;

pub mod counter;
pub mod sequence;
pub mod ringbuf;
pub mod channel;

macro_rules! specialize {
    ($(
        mod $name:ident<$S:ty, $R:ty, $E:ty>;
    )*) => ($(
        pub mod $name {
            #![allow(missing_docs)]
            #![allow(unused_imports)]

            use super::*;
            use channel as chan;

            pub use channel::{SendError, SendErrorKind, ReceiveError};
            pub type Sender<T> = chan::Sender<$S, $R, $E, T>;
            pub type Receiver<T> = chan::Receiver<$S, $R, $E, T>;

            pub fn channel<T: Send>(capacity: usize) -> (Sender<T>, Receiver<T>) {
                chan::channel(capacity)
            }
        }
    )*);
}

use sequence::{Owned, Shared};

specialize! {
    mod spsc<Owned, Owned, ()>;
    mod mpsc<Shared, Owned, ()>;
    mod spmc<Owned, Shared, ()>;
    mod mpmc<Shared, Shared, ()>;
}
