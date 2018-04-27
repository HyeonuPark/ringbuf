
use std::ops::Index;
use std::cell::Cell;
use std::sync::Arc;

// pub mod owned;

use counter::Counter;
use blocker::{Blocker, BlockerStack};

pub struct Bucket<T> {
    blockers: BlockerStack,
    inner: Cell<Option<T>>,
}

pub struct Slot<'a, 'b, 'c, S> where S: Sequence + 'a, S::Cache: 'b, S::Item: 'c {
    seq: &'a S,
    cache: &'b mut S::Cache,
    bucket: &'c Bucket<S::Item>,
    count: Counter,
}

pub struct TrySlot<'a, 'b, 'c, S, L> where S: Sequence + 'a, S::Cache: 'b, S::Item: 'c, L: Limit {
    slot: Slot<'a, 'b, 'c, S>,
    limit: L,
}

pub trait Sequence: Sized {
    type Cache;
    type Item;

    fn and_cache() -> (Self, Self::Cache);

    fn count(&self) -> Counter;

    fn try_claim<L: Limit>(&self, cache: &mut Self::Cache, limit: L) -> Option<Counter>;

    fn claim<L: Limit>(&self, cache: &mut Self::Cache, limit: L) -> Result<Counter, Counter>;

    fn commit(&self, cache: &mut Self::Cache, count: Counter);

    fn try_advance<'a, 'b, 'c, L, B>(
        &'a self,
        cache: &'b mut Self::Cache,
        limit: L,
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

    fn advance<'a, 'b, 'c, L, B>(
        &'a self,
        cache: &'b mut Self::Cache,
        limit: L,
        buf: &'c B,
        blocker: &mut Blocker,
    ) -> Result<Slot<'a, 'b, 'c, Self>, TrySlot<'a, 'b, 'c, Self, L>> where
        L: Limit,
        B: Index<Counter, Output=Bucket<Self::Item>>,
    {
        match self.claim(cache, limit.clone()) {
            Ok(count) => Ok(Slot {
                seq: self,
                bucket: &buf[count],
                count,
                cache,
            }),
            Err(count) => {
                let bucket = &buf[count];
                bucket.blockers.push(blocker);

                Err(TrySlot {
                    limit,
                    slot: Slot {
                        seq: self,
                        bucket,
                        count,
                        cache,
                    }
                })
            }
        }
    }
}

pub trait Limit: Clone {
    fn count(&self) -> Counter;
}

pub trait Shared: Sequence {
    fn new_cache(&self) -> Self::Cache;
}

impl<T> Bucket<T> {
    pub fn get(&self) -> T {
        self.inner.take()
            .expect("Bucket::get should not be called on empty slot")
    }

    pub fn set(&self, item: T) {
        match self.inner.replace(Some(item)) {
            Some(_) => panic!("Bucket::set should not be called on non-empty Bucket"),
            None => {}
        }
    }

    pub fn notify(&self) {
        if let Some(blocker) = self.blockers.pop() {
            let blocker = unsafe { Arc::from_raw(blocker.as_ptr()) };
            blocker.unblock();
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

impl<'a, 'b, 'c, S, L> TrySlot<'a, 'b, 'c, S, L> where S: Sequence, L: Limit {
    fn check(&self) -> bool {
        self.limit.count() > self.slot.count
    }

    pub fn try_get(self) -> Result<S::Item, Self> {
        if self.check() {
            Ok(self.slot.get())
        } else {
            Err(self)
        }
    }

    pub fn try_set(self, item: S::Item) -> Result<(), (Self, S::Item)> {
        if self.check() {
            Ok(self.slot.set(item))
        } else {
            Err((self, item))
        }
    }
}
