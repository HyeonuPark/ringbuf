
use counter::{Counter, AtomicCounter};
use sequence::{Sequence, Limit};

#[derive(Debug)]
pub struct Owned {
    count: AtomicCounter,
}

#[derive(Debug)]
pub struct Cache {
    count: Counter,
    limit: Counter,
}

impl Sequence for Owned {
    type Cache = Cache;

    fn new() -> (Self, Cache) {
        (
            Owned {
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
        debug_assert!(cache.count <= cache.limit);

        if cache.count >= cache.limit {
            let recent_limit = limit.limit();
            debug_assert!(recent_limit >= cache.limit);
            cache.limit = recent_limit;
        }

        if cache.count >= cache.limit {
            None
        } else {
            Some(cache.count)
        }
    }

    fn commit(&self, cache: &mut Cache, count: Counter) {
        let prev = self.count.incr(1);
        debug_assert_eq!(prev, count);
        debug_assert_eq!(cache.count, count);
        cache.count = count + 1;
    }
}
