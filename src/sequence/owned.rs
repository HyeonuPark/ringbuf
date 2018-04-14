
use sequence::Sequence;
use counter::{Counter, AtomicCounter};

#[derive(Debug, Default)]
pub struct Owned {
    count: AtomicCounter,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Cache {
    count: Counter,
    limit: Counter,
}

impl Sequence for Owned {
    type Cache = Cache;

    fn count(&self) -> Counter {
        self.count.get()
    }

    fn claim<L: Sequence>(&self, cache: &mut Cache, limit: &L) -> Option<Counter> {
        debug_assert!(cache.limit >= cache.count);

        if cache.limit == cache.count {
            // try push limit
            cache.limit = limit.count();
        }

        debug_assert!(cache.limit >= cache.count);

        if cache.limit == cache.count {
            // failed to push limit
            None
        } else {
            let prev_count = self.count.incr(1);
            debug_assert_eq!(prev_count, cache.count);
            cache.count = prev_count + 1;

            Some(prev_count)
        }
    }

    fn commit(&self, _cache: &mut Cache, _index: Counter) {
        // no-op
    }
}
