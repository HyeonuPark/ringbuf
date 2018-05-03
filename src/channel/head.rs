
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicBool};

use counter::Counter;
use sequence::{Sequence, Limit};
use buffer::BufInfo;

use super::half::HeadHalf;

#[derive(Debug)]
pub struct Head<S: Sequence, R: Sequence> {
    sender: S,
    sender_count: AtomicUsize,
    receiver: R,
    receiver_count: AtomicUsize,
    is_closed: AtomicBool,
}

#[derive(Debug)]
pub struct SenderHead<S: Sequence, R: Sequence> {
    pub head: Arc<Head<S, R>>,
    pub capacity: usize,
}

#[derive(Debug)]
pub struct ReceiverHead<S: Sequence, R: Sequence> {
    pub head: Arc<Head<S, R>>,
}

impl<S: Sequence, R: Sequence> Head<S, R> {
    pub fn new(sender: S, receiver: R) -> Self {
        Head {
            sender,
            sender_count: AtomicUsize::new(0),
            receiver,
            receiver_count: AtomicUsize::new(0),
            is_closed: AtomicBool::new(false),
        }
    }
}

impl<S: Sequence, R: Sequence> BufInfo for Arc<Head<S, R>> {
    fn range(&self) -> (Counter, Counter) {
        (self.receiver.count(), self.sender.count())
    }
}

impl<S: Sequence, R: Sequence> HeadHalf for SenderHead<S, R> {
    type Seq = S;

    fn seq(&self) -> &S {
        &self.head.sender
    }

    fn count(&self) -> &AtomicUsize {
        &self.head.sender_count
    }

    fn is_closed(&self) -> &AtomicBool {
        &self.head.is_closed
    }
}

impl<S: Sequence, R: Sequence> Limit for SenderHead<S, R> {
    fn limit(&self) -> Counter {
        self.head.receiver.count() + self.capacity
    }
}

impl<S: Sequence, R: Sequence> Clone for SenderHead<S, R> {
    fn clone(&self) -> Self {
        SenderHead {
            head: self.head.clone(),
            capacity: self.capacity,
        }
    }
}

impl<S: Sequence, R: Sequence> HeadHalf for ReceiverHead<S, R> {
    type Seq = R;

    fn seq(&self) -> &R {
        &self.head.receiver
    }

    fn count(&self) -> &AtomicUsize {
        &self.head.receiver_count
    }

    fn is_closed(&self) -> &AtomicBool {
        &self.head.is_closed
    }
}

impl<S: Sequence, R: Sequence> Limit for ReceiverHead<S, R> {
    fn limit(&self) -> Counter {
        self.head.sender.count()
    }
}

impl<S: Sequence, R: Sequence> Clone for ReceiverHead<S, R> {
    fn clone(&self) -> Self {
        ReceiverHead {
            head: self.head.clone(),
        }
    }
}
