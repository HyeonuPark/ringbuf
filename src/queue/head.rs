
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::marker::PhantomData;

use role;
use counter::{Counter, CounterRange, AtomicCounter};
use sequence::{Sequence, Limit};
use buffer::BufRange;

use queue::half::{Half, HeadHalf};

#[derive(Debug)]
pub(crate) struct Head<S: Sequence, R: Sequence> {
    sender: S,
    receiver: R,
    sender_count: AtomicUsize,
    receiver_count: AtomicUsize,
}

#[derive(Debug)]
pub(crate) struct SenderHead<S: Sequence, R: Sequence, T> {
    head: Arc<Head<S, R>>,
    capacity: usize,
    role: PhantomData<role::Send<T>>,
}

#[derive(Debug)]
pub(crate) struct ReceiverHead<S: Sequence, R: Sequence, T> {
    head: Arc<Head<S, R>>,
    role: PhantomData<role::Receive<T>>,
}

pub(crate) type SenderHalf<S, R, T> = Half<Arc<Head<S, R>>, SenderHead<S, R, T>, T>;
pub(crate) type ReceiverHalf<S, R, T> = Half<Arc<Head<S, R>>, ReceiverHead<S, R, T>, T>;

impl<S: Sequence, R: Sequence> Head<S, R> {
    pub fn new(sender: S, receiver: R) -> Arc<Self> {
        Arc::new(Head {
            sender,
            receiver,
            sender_count: 0.into(),
            receiver_count: 0.into(),
        })
    }
}

impl<S: Sequence, R: Sequence> Head<S, R> {
    pub fn close(&self) {
        self.sender.counter().close();
    }
}

impl<S: Sequence, R: Sequence> BufRange for Arc<Head<S, R>> {
    fn range(&self) -> CounterRange {
        let sender_last = self.sender.fetch_last();
        let receiver_last = self.receiver.fetch_last();

        Counter::range(sender_last, receiver_last)
    }
}

impl<S: Sequence, R: Sequence, T> SenderHead<S, R, T> {
    pub fn new(head: Arc<Head<S, R>>, capacity: usize) -> Self {
        SenderHead {
            head,
            capacity,
            role: PhantomData,
        }
    }
}

impl<S: Sequence, R: Sequence, T> HeadHalf for SenderHead<S, R, T> {
    type Seq = S;
    type Role = role::Send<T>;

    fn seq(&self) -> &S {
        &self.head.sender
    }

    fn amount(&self) -> &AtomicUsize {
        &self.head.sender_count
    }

    fn close_counter(&self) -> &AtomicCounter {
        self.head.sender.counter()
    }
}

impl<S: Sequence, R: Sequence, T> Limit for SenderHead<S, R, T> {
    fn count(&self) -> Counter {
        self.head.receiver.fetch_last() + self.capacity
    }
}

impl<S: Sequence, R: Sequence, T> Clone for SenderHead<S, R, T> {
    fn clone(&self) -> Self {
        SenderHead {
            head: Arc::clone(&self.head),
            capacity: self.capacity,
            role: PhantomData,
        }
    }
}

impl<S: Sequence, R: Sequence, T> ReceiverHead<S, R, T> {
    pub fn new(head: Arc<Head<S, R>>) -> Self {
        ReceiverHead {
            head,
            role: PhantomData,
        }
    }
}

impl<S: Sequence, R: Sequence, T> HeadHalf for ReceiverHead<S, R, T> {
    type Seq = R;
    type Role = role::Receive<T>;

    fn seq(&self) -> &R {
        &self.head.receiver
    }

    fn amount(&self) -> &AtomicUsize {
        &self.head.receiver_count
    }

    fn close_counter(&self) -> &AtomicCounter {
        self.head.sender.counter()
    }
}

impl<S: Sequence, R: Sequence, T> Limit for ReceiverHead<S, R, T> {
    fn count(&self) -> Counter {
        self.head.sender.fetch_last()
    }
}

impl<S: Sequence, R: Sequence, T> Clone for ReceiverHead<S, R, T> {
    fn clone(&self) -> Self {
        ReceiverHead {
            head: Arc::clone(&self.head),
            role: PhantomData,
        }
    }
}
