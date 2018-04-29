
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::ops::Drop;
use std::cell::Cell;
use std::thread;

use counter::Counter;
use buffer::{Buffer, BufInfo};
use blocker::{Blocker, BlockerNode, BlockKind};
use sequence::{Sequence, Limit, Shared, Slot};

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
pub struct Half<B: BufInfo, H: HeadHalf<Seq=S>, S: Sequence> {
    buf: Buffer<B, S::Item>,
    head: H,
    cache: S::Cache,
    is_closed_cache: Cell<bool>,
    block_node: BlockerNode,
    block_kind: Box<BlockKind>,

    // for sanity check
    #[cfg(debug_assertions)]
    is_blocked: bool,
}

pub trait HeadHalf: Clone + Limit {
    type Seq: Sequence;

    fn seq(&self) -> &Self::Seq;
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
    type Seq = S;

    fn seq(&self) -> &S {
        &self.0.sender_seq
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
    type Seq = R;

    fn seq(&self) -> &R {
        &self.0.receiver_seq
    }

    fn amount(&self) -> &AtomicUsize {
        &self.0.receiver_count
    }

    fn is_closed(&self) -> &AtomicBool {
        &self.0.is_closed
    }
}

macro_rules! is_closed {
    ($this:expr) => ({
        if $this.is_closed_cache.get() {
            true
        } else if $this.head.is_closed().load(Ordering::Acquire) {
            $this.is_closed_cache.set(true);
            true
        } else {
            false
        }
    });
}

macro_rules! advance {
    ($this:expr) => ({
        assert!($this.block_node.set(&mut $this.block_kind));

        $this.head.seq().advance(
            &mut $this.cache,
            &$this.head,
            &$this.buf,
            $this.block_node.clone(),
        )
    });
}

impl<B: BufInfo, H: HeadHalf<Seq=S>, S: Sequence> Half<B, H, S> {
    pub fn new(buf: Buffer<B, S::Item>, head: H, cache: S::Cache) -> Self {
        Half {
            buf,
            head,
            cache,
            is_closed_cache: false.into(),
            block_node: Blocker::new(),
            block_kind: Box::new(BlockKind::Nothing),
            #[cfg(debug_assertions)]
            is_blocked: false,
        }
    }

    pub fn is_closed(&self) -> bool {
        is_closed!(self)
    }

    pub fn close(&mut self) {
        if self.is_closed_cache.get() {
            return;
        }

        self.is_closed_cache.set(true);
        self.head.is_closed().store(true, Ordering::Release);
    }

    pub fn try_advance(&mut self) -> Option<Slot<S>> {
        self.head.seq().try_advance(
            &mut self.cache,
            &self.head,
            &self.buf,
        )
    }

    pub fn sync_advance(&mut self) -> Option<Slot<S>> {
        *self.block_kind = BlockKind::Sync(thread::current());
        let mut try_slot = advance!(self);

        loop {
            match try_slot.try_unwrap() {
                Ok(slot) => return Some(slot),
                Err(try_again) => {
                    try_slot = try_again;
                    thread::park();
                }
            }

            if is_closed!(self) {
                return None;
            }
        }
    }
}

impl<B, H, S> Clone for Half<B, H, S> where
    B: BufInfo,
    H: HeadHalf<Seq=S>,
    S: Shared + Sequence,
{
    fn clone(&self) -> Self {
        self.head.amount().fetch_add(1, Ordering::Relaxed);

        Half::new(
            self.buf.clone(),
            self.head.clone(),
            self.head.seq().new_cache(&self.head),
        )
    }
}

impl<B, H, S> Drop for Half<B, H, S> where
    B: BufInfo,
    H: HeadHalf<Seq=S>,
    S: Sequence,
{
    fn drop(&mut self) {
        if self.head.amount().fetch_sub(1, Ordering::Release) == 1 {
            assert_eq!(self.head.amount().load(Ordering::Acquire), 0);
            self.close();
            self.buf.notify_all();
        }
    }
}
