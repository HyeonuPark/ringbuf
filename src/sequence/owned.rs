
use counter::{Counter, AtomicCounter};
use sequence::{Sequence, Limit};

#[derive(Debug, Default)]
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

    fn cache<L: Limit>(&self, limit: &L) -> Cache {
        Cache {
            count: self.count(),
            limit: limit.count(),
        }
    }

    fn count(&self) -> Counter {
        self.count.fetch()
    }

    fn try_claim<L: Limit>(&self, cache: &mut Cache, limit: &L) -> Option<Counter> {
        debug_assert!(cache.count <= cache.limit);

        if cache.count >= cache.limit {
            let recent_limit = limit.count();
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
