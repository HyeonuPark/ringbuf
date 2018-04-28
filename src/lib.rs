//! Thread safe bounded channels based on ring buffer.
//!
//! This crate provides channels that can be used to communicate
//! between asynchronous tasks.
//!
//! Channels are based on fixed-sized ring buffer. Send operations simply fail
//! if backing buffer is full, and you can get back message you sent from error.

pub mod intrusive;
pub mod counter;

pub mod blocker;
pub mod buffer;
pub mod sequence;

pub mod queue;
