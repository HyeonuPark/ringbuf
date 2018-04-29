
use std::marker::PhantomData;

use counter::{Counter, AtomicCounter};
use sequence::{Sequence, Limit, Shared};

#[derive(Debug)]
pub struct Preemptive<T> {
    _marker: PhantomData<*mut T>,
    count: AtomicCounter,
    claimed: AtomicCounter,
}

unsafe impl<T: Send> Sync for Preemptive<T> {}

#[derive(Debug)]
pub struct Cache {
    limit: Counter,
}

impl<T> Sequence for Preemptive<T> {
    type Item = T;
    type Cache = Cache;

    fn new() -> (Self, Cache) {
        (
            Preemptive {
                _marker: PhantomData,
                count: AtomicCounter::new(),
                claimed: AtomicCounter::new(),
            },
            Cache {
                limit: Counter::new(0),
            },
        )
    }

    fn count(&self) -> Counter {
        self.count.fetch()
    }

    fn try_claim<L: Limit>(&self, cache: &mut Cache, limit: &L) -> Option<Counter> {
        let mut claimed = self.claimed.fetch();

        loop {
            if cache.limit <= claimed {
                let recent_limit = limit.count();
                debug_assert!(recent_limit >= cache.limit);
                cache.limit = recent_limit;
            }

            if cache.limit <= claimed {
                return None;
            }

            match self.claimed.cond_swap(claimed, claimed + 1) {
                Ok(()) => return Some(claimed),
                Err(prev) => claimed = prev,
            }
        }
    }

    fn claim(&self, _cache: &mut Cache) -> Counter {
        self.claimed.incr(1)
    }

    fn commit(&self, _cache: &mut Cache, count: Counter) {
        let next = count + 1;

        loop {
            if let Ok(()) = self.count.cond_swap(count, next) {
                return;
            }
        }
    }
}

impl<T> Shared for Preemptive<T> {
    fn new_cache<L: Limit>(&self, limit: &L) -> Cache {
        Cache {
            limit: limit.count(),
        }
    }
}
