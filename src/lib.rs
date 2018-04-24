//! Thread safe bounded channels based on ring buffer.
//!
//! This crate provides channels that can be used to communicate
//! between asynchronous tasks.
//!
//! Channels are based on fixed-sized ring buffer. Send operations simply fail
//! if backing buffer is full, and you can get back message you sent from error.

#![deny(missing_docs)]

#[macro_use]
extern crate defmac;

extern crate crossbeam;
// not for now
// extern crate futures;

#[cfg(test)]
extern crate rand;

pub mod counter;
pub mod sequence;
pub mod ringbuf;
pub mod queue;
