
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

    fn count(&self) -> Counter {
        self.count.fetch()
    }

    fn cache<'a, L: Limit<'a>>(&self, limit: L) -> Cache {
        Cache {
            count: self.count(),
            limit: limit.count(),
        }
    }

    fn try_claim<'a, L: Limit<'a>>(&self, cache: &mut Cache, limit: L) -> Option<Counter> {
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

    fn claim<'a, L: Limit<'a>>(&self, cache: &mut Cache, limit: L) -> Result<Counter, Counter> {
        debug_assert!(cache.limit >= cache.count);

        if cache.limit == cache.count {
            let recent_limit = limit.count();
            debug_assert!(recent_limit >= cache.limit);
            cache.limit = recent_limit;
        }

        if cache.limit == cache.count {
            Err(cache.count)
        } else {
            Ok(cache.count)
        }
    }

    fn commit(&self, cache: &mut Cache, count: Counter) {
        debug_assert_eq!(count, cache.count);
        let prev_count = self.count.incr(1);
        debug_assert_eq!(count, prev_count);
        cache.count = count + 1;
    }
}
