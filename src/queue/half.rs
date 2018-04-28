
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::ops::Drop;
use std::cell::Cell;

use counter::Counter;
use buffer::{Buffer, BufInfo};
use blocker::{Blocker, BlockerNode, BlockKind};
use sequence::{Sequence, Limit, Shared, Bucket, Slot, TrySlot};

#[derive(Debug)]
pub struct Head<S: Sequence, R: Sequence> {
    sender_seq: S,
    sender_count: AtomicUsize,
    receiver_seq: R,
    receiver_count: AtomicUsize,
    is_closed: AtomicBool,
}

impl<S: Sequence, R: Sequence> Head<S, R> {
    pub fn new(sender_seq: S, receiver_seq: R) -> Self {
        Head {
            sender_seq,
            sender_count: 1.into(),
            receiver_seq,
            receiver_count: 1.into(),
            is_closed: false.into(),
        }
    }
}

impl<S: Sequence, R: Sequence> BufInfo for Arc<Head<S, R>> {
    fn start(&self) -> Counter {
        self.receiver_seq.count()
    }

    fn end(&self) -> Counter {
        self.sender_seq.count()
    }
}

#[derive(Debug)]
pub struct SenderHead<S: Sequence, R: Sequence>(pub Arc<Head<S, R>>, pub usize); // capacity

#[derive(Debug)]
pub struct ReceiverHead<S: Sequence, R: Sequence>(pub Arc<Head<S, R>>);

#[derive(Debug)]
pub struct Half<B: BufInfo, H: HeadHalf<Near=S>, S: Sequence<Item=T>, T: Send> {
    buf: Buffer<B, Bucket<T>>,
    head: H,
    cache: S::Cache,
    node: BlockerNode,
    blocked: Box<BlockKind>,
    is_closed_cache: Cell<bool>,
}

pub trait HeadHalf: Clone + Limit {
    type Near: Sequence;
    type Far: Sequence;

    fn near(&self) -> &Self::Near;
    fn far(&self) -> &Self::Far;
    fn amount(&self) -> &AtomicUsize;
    fn is_closed(&self) -> &AtomicBool;
}

impl<S: Sequence, R: Sequence> Clone for SenderHead<S, R> {
    fn clone(&self) -> Self {
        SenderHead(self.0.clone(), self.1)
    }
}

impl<S: Sequence, R: Sequence> Limit for SenderHead<S, R> {
    fn count(&self) -> Counter {
        self.0.receiver_seq.count() + self.1
    }
}

impl<S: Sequence, R: Sequence> HeadHalf for SenderHead<S, R> {
    type Near = S;
    type Far = R;

    fn near(&self) -> &S {
        &self.0.sender_seq
    }

    fn far(&self) -> &R {
        &self.0.receiver_seq
    }

    fn amount(&self) -> &AtomicUsize {
        &self.0.sender_count
    }

    fn is_closed(&self) -> &AtomicBool {
        &self.0.is_closed
    }
}

impl<S: Sequence, R: Sequence> Clone for ReceiverHead<S, R> {
    fn clone(&self) -> Self {
        ReceiverHead(self.0.clone())
    }
}

impl<S: Sequence, R: Sequence> Limit for ReceiverHead<S, R> {
    fn count(&self) -> Counter {
        self.0.sender_seq.count()
    }
}

impl<S: Sequence, R: Sequence> HeadHalf for ReceiverHead<S, R> {
    type Near = R;
    type Far = S;

    fn near(&self) -> &R {
        &self.0.receiver_seq
    }

    fn far(&self) -> &S {
        &self.0.sender_seq
    }

    fn amount(&self) -> &AtomicUsize {
        &self.0.receiver_count
    }

    fn is_closed(&self) -> &AtomicBool {
        &self.0.is_closed
    }
}

impl<B: BufInfo, H: HeadHalf<Near=S>, S: Sequence<Item=T>, T: Send> Half<B, H, S, T> {
    pub fn new(buf: Buffer<B, Bucket<T>>, head: H, cache: S::Cache) -> Self {
        Half {
            buf,
            head,
            cache,
            node: Blocker::new(),
            blocked: Box::new(BlockKind::Nothing),
            is_closed_cache: false.into(),
        }
    }

    pub fn is_closed(&self) -> bool {
        if self.is_closed_cache.get() {
            return true;
        }

        if self.head.is_closed().load(Ordering::Acquire) {
            self.is_closed_cache.set(true);
            true
        } else {
            false
        }
    }

    pub fn close(&mut self) {
        if self.is_closed_cache.get() {
            return;
        }

        self.is_closed_cache.set(true);
        self.head.is_closed().store(true, Ordering::Release);
    }

    pub fn try_advance(&mut self) -> Option<Slot<S>> {
        self.head.near().try_advance(
            &mut self.cache,
            &self.head,
            &self.buf,
        )
    }
}

impl<B, H, S, T> Clone for Half<B, H, S, T> where
    B: BufInfo,
    H: HeadHalf<Near=S>,
    S: Shared + Sequence<Item=T>,
    T: Send,
{
    fn clone(&self) -> Self {
        self.head.amount().fetch_add(1, Ordering::Relaxed);

        Half::new(
            self.buf.clone(),
            self.head.clone(),
            self.head.near().new_cache(&self.head),
        )
    }
}

impl<B, H, S, T> Drop for Half<B, H, S, T> where
    B: BufInfo,
    H: HeadHalf<Near=S>,
    S: Sequence<Item=T>,
    T: Send,
{
    fn drop(&mut self) {
        if self.head.amount().fetch_sub(1, Ordering::Release) == 1 {
            self.close();
            // TODO: notify blocked opposites
            unimplemented!()
        }
    }
}
