
use std::ops::Index;
use std::cell::UnsafeCell;
use std::{ptr, mem};
#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

pub mod owned;
pub mod preemptive;

use counter::Counter;
use blocker::{Blocker, BlockerNode, BlockerContainer};

#[derive(Debug)]
pub struct Bucket<T> {
    blockers: BlockerContainer,
    inner: UnsafeCell<T>,
    #[cfg(debug_assertions)]
    has_inner: AtomicBool,
}

#[derive(Debug)]
pub struct Slot<'a, 'b, 'c, S> where S: Sequence + 'a, S::Cache: 'b, S::Item: 'c {
    seq: &'a S,
    cache: &'b mut S::Cache,
    bucket: &'c Bucket<S::Item>,
    count: Counter,
}

pub struct TrySlot<'a, 'b, 'c, 'd, S, L> where
    S: Sequence + 'a, S::Cache: 'b, S::Item: 'c, L: Limit + 'd
{
    limit: &'d L,
    blocker: BlockerNode,
    slot: Slot<'a, 'b, 'c, S>,
}

pub trait Sequence: Sized {
    type Item;
    type Cache: ::std::fmt::Debug;

    fn new() -> (Self, Self::Cache);

    fn count(&self) -> Counter;

    fn try_claim<L: Limit>(&self, cache: &mut Self::Cache, limit: &L) -> Option<Counter>;

    fn claim(&self, cache: &mut Self::Cache) -> Counter;

    fn commit(&self, cache: &mut Self::Cache, count: Counter);

    fn try_advance<'a, 'b, 'c, L, B>(
        &'a self,
        cache: &'b mut Self::Cache,
        limit: &L,
        buf: &'c B,
    ) -> Option<Slot<'a, 'b, 'c, Self>> where
        L: Limit,
        B: Index<Counter, Output=Bucket<Self::Item>>,
    {
        match self.try_claim(cache, limit) {
            None => None,
            Some(count) => Some(Slot {
                seq: self,
                bucket: &buf[count],
                count,
                cache,
            }),
        }
    }

    fn advance<'a, 'b, 'c, 'd, L, B>(
        &'a self,
        cache: &'b mut Self::Cache,
        limit: &'d L,
        buf: &'c B,
        blocker: BlockerNode,
    ) -> TrySlot<'a, 'b, 'c, 'd, Self, L> where
        L: Limit,
        B: Index<Counter, Output=Bucket<Self::Item>>,
    {
        let count = self.claim(cache);

        TrySlot {
            limit,
            blocker,
            slot: Slot {
                seq: self,
                bucket: &buf[count],
                count,
                cache,
            },
        }
    }
}

pub trait Limit: Clone {
    fn count(&self) -> Counter;
}

pub trait Shared: Sequence {
    fn new_cache<L: Limit>(&self, limit: &L) -> Self::Cache;
}

impl<T> Bucket<T> {
    pub fn get(&self) -> T {
        #[cfg(debug_assertions)]
        assert!(self.has_inner.load(SeqCst),
            "Bucket::get should not be called on empty slot");

        let res = unsafe {
            ptr::read(self.inner.get())
        };

        #[cfg(debug_assertions)]
        self.has_inner.store(false, SeqCst);

        res
    }

    pub fn set(&self, item: T) {
        #[cfg(debug_assertions)]
        assert!(!self.has_inner.load(SeqCst),
            "Bucket::set should not be called on non-empty Bucket");

        unsafe {
            ptr::write(self.inner.get(), item);
        }

        #[cfg(debug_assertions)]
        self.has_inner.store(true, SeqCst);
    }

    pub fn notify(&self) {
        if let Some(blocker) = self.blockers.pop() {
            blocker.unblock();
        }
    }
}

impl<T> Default for Bucket<T> {
    fn default() -> Self {
        Bucket {
            blockers: Blocker::container(),
            inner: UnsafeCell::new(unsafe { mem::uninitialized() }),
            #[cfg(debug_assertions)]
            has_inner: false.into(),
        }
    }
}

impl<'a, 'b, 'c, S> Slot<'a, 'b, 'c, S> where S: Sequence {
    pub fn get(self) -> S::Item {
        let res = self.bucket.get();
        self.seq.commit(self.cache, self.count);
        self.bucket.notify();
        res
    }

    pub fn set(self, item: S::Item) {
        self.bucket.set(item);
        self.seq.commit(self.cache, self.count);
        self.bucket.notify();
    }
}

impl<'a, 'b, 'c, 'd, S, L> TrySlot<'a, 'b, 'c, 'd, S, L> where S: Sequence, L: Limit {
    pub fn try_unwrap(self) -> Result<Slot<'a, 'b, 'c, S>, Self> {
        if self.limit.count() > self.slot.count {
            Ok(self.slot)
        } else {
            self.slot.bucket.blockers.push(self.blocker.clone());
            Err(self)
        }
    }
}
