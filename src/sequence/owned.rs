
use std::sync::atomic::{AtomicBool, Ordering};

use counter::{Counter, AtomicCounter};
use sequence::{Sequence, Limit, CacheError, CommitError};

#[derive(Debug, Default)]
pub struct Owned {
    count: AtomicCounter,
    has_cache: AtomicBool,
}

#[derive(Debug)]
pub struct Cache {
    count: Counter,
    limit: Counter,
}

impl Sequence for Owned {
    type Cache = Cache;

    fn cache<L: Limit>(&self, limit: &L) -> Result<Cache, CacheError> {
        // Owned sequence can have up to single cache
        if self.has_cache.fetch_or(true, Ordering::Release) {
            return Err(CacheError::NotAvailable);
        }

        match self.count.fetch() {
            Ok(count) => Ok(Cache {
                count,
                limit: limit.count(),
            }),
            Err(_) => Err(CacheError::SeqClosed,)
        }
    }

    fn counter(&self) -> &AtomicCounter {
        &self.count
    }

    fn claim<L: Limit>(&self, cache: &mut Cache, limit: &L) -> Option<Counter> {
        debug_assert!(cache.count <= cache.limit);

        if cache.count == cache.limit {
            let recent_limit = limit.count();
            debug_assert!(recent_limit >= cache.limit);
            cache.limit = recent_limit;
        }

        if cache.count == cache.limit {
            None
        } else {
            let count = cache.count;
            cache.count = count + 1;
            Some(count)
        }
    }

    fn commit(&self, cache: &mut Cache, count: Counter) -> Result<(), CommitError> {
        match self.count.incr() {
            None => Err(CommitError),
            Some(prev) => {
                debug_assert_eq!(prev, count);
                debug_assert_eq!(cache.count, prev + 1);
                Ok(())
            }
        }
    }
}
