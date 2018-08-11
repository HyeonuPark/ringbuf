
use std::sync::atomic::Ordering;

use counter::{Counter, AtomicCounter};
use sequence::{Sequence, Limit, MultiCache, CacheError, CommitError};

#[derive(Debug, Default)]
pub struct Shared {
    claimed: AtomicCounter,
    count: AtomicCounter,
}

#[derive(Debug)]
pub struct Cache {
    limit: Counter,
}

impl MultiCache for Shared {}

impl Sequence for Shared {
    type Cache = Cache;

    fn cache<L: Limit>(&self, limit: &L) -> Result<Cache, CacheError> {
        match self.count.fetch() {
            Ok(_) => Ok(Cache {
                limit: limit.count(),
            }),
            Err(_) => Err(CacheError::SeqClosed,)
        }
    }

    fn counter(&self) -> &AtomicCounter {
        &self.count
    }

    fn claim<L: Limit>(&self, cache: &mut Cache, limit: &L) -> Option<Counter> {
        // Increase claimed counter
        let claimed = self.claimed.incr()?;

        loop {
            // Fetch recent limit if cached limit is lower than claimed
            if claimed >= cache.limit {
                let recent_limit = limit.count();
                debug_assert!(recent_limit >= cache.limit);
                cache.limit = recent_limit;
            }

            // Revert if limit is lower than claimed
            if claimed >= cache.limit {
                match self.claimed.comp_swap(claimed + 1, claimed, Ordering::AcqRel) {
                    Ok(()) => return None,
                    Err(prev) => {
                        // Recheck limit if revert is failed
                        debug_assert!(prev.map_or(true, |prev| prev > claimed + 1));
                        continue;
                    }
                }
            }

            return Some(claimed);
        }
    }

    fn commit(&self, _cache: &mut Cache, count: Counter) -> Result<(), CommitError> {
        loop {
            match self.count.comp_swap(count, count + 1, Ordering::AcqRel) {
                Ok(()) => return Ok(()),
                Err(Some(_)) => continue, // Other thread modified it. Retry
                Err(None) => return Err(CommitError), // Sequence closed.
            }
        }
    }
}
