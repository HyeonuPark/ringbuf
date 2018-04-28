
use std::marker::PhantomData;

use counter::{Counter, AtomicCounter};
use sequence::{Sequence, Limit};

#[derive(Debug)]
pub struct Owned<T> {
    _marker: PhantomData<*mut T>,
    count: AtomicCounter,
}

unsafe impl<T: Send> Sync for Owned<T> {}

#[derive(Debug)]
pub struct Cache {
    count: Counter,
    limit: Counter,
}

impl<T> Sequence for Owned<T> {
    type Item = T;
    type Cache = Cache;

    fn new() -> (Self, Cache) {
        (
            Owned {
                _marker: PhantomData,
                count: AtomicCounter::new(),
            },
            Cache {
                count: Counter::new(0),
                limit: Counter::new(0),
            },
        )
    }

    fn count(&self) -> Counter {
        self.count.fetch()
    }

    fn try_claim<L: Limit>(&self, cache: &mut Cache, limit: &L) -> Option<Counter> {
        debug_assert!(cache.limit >= cache.count);

        if cache.limit == cache.count {
            let recent_limit = limit.count();
            debug_assert!(recent_limit >= cache.limit);
            cache.limit = recent_limit;
        }

        if cache.limit == cache.count {
            None
        } else {
            Some(cache.count)
        }
    }

    fn claim<L: Limit>(&self, cache: &mut Cache, limit: &L) -> Result<Counter, Counter> {
        self.try_claim(cache, limit).ok_or(cache.count)
    }

    fn commit(&self, cache: &mut Cache, count: Counter) {
        let prev = self.count.incr(1);
        debug_assert_eq!(prev, count);
        debug_assert_eq!(prev, cache.count);
        debug_assert!(cache.limit >= cache.count);
        cache.count = prev + 1;
    }
}
