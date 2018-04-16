use sequence::{Sequence, Limit};
use counter::{Counter, AtomicCounter};

/// Owned, unique sequence.
///
/// Owner of this sequence has exclusive access to this end of the channel.
#[derive(Debug, Default)]
pub struct Owned {
    count: AtomicCounter,
}

/// Cache for owned sequence.
#[derive(Debug, Default, Clone, Copy)]
pub struct Cache {
    count: Counter,
    limit: Counter,
}

impl Sequence for Owned {
    type Cache = Cache;

    fn count(&self) -> Counter {
        self.count.fetch()
    }

    fn claim<L: Limit>(&self, cache: &mut Cache, limit: L) -> Option<Counter> {
        debug_assert!(cache.limit >= cache.count);

        if cache.limit <= cache.count {
            // try to push limit
            cache.limit = limit.count();
        }

        debug_assert!(cache.limit >= cache.count);

        if cache.limit <= cache.count {
            // failed to push limit
            None
        } else {
            Some(cache.count)
        }
    }

    fn commit(&self, cache: &mut Cache, index: Counter) {
        let prev_count = self.count.incr(1);
        debug_assert_eq!(prev_count, index);
        debug_assert_eq!(prev_count, cache.count);
        cache.count = prev_count + 1;
    }
}
