
use counter::{Counter, AtomicCounter};
use sequence::{Sequence, Limit};

#[derive(Debug, Default)]
pub struct Shared {
    count: AtomicCounter,
    claimed: AtomicCounter,
}

#[derive(Debug, Clone)]
pub struct Cache {
    limit: Counter,
}

impl Sequence for Shared {
    type Cache = Cache;

    fn count(&self) -> Counter {
        self.count.fetch()
    }

    fn cache<'a, L: Limit<'a>>(&self, limit: L) -> Cache {
        Cache {
            limit: limit.count(),
        }
    }

    fn try_claim<'a, L: Limit<'a>>(&self, cache: &mut Cache, limit: L) -> Option<Counter> {
        let mut prev = self.claimed.fetch();

        loop {
            if cache.limit <= prev {
                let recent_limit = limit.count();
                debug_assert!(recent_limit >= cache.limit);
                cache.limit = recent_limit;
            }

            if cache.limit <= prev {
                return None;
            }

            match self.claimed.cond_swap(prev, prev + 1) {
                Ok(()) => return Some(prev),
                Err(changed) => {
                    debug_assert!(changed > prev);
                    prev = changed;
                }
            }
        }
    }

    fn claim<'a, L: Limit<'a>>(&self, cache: &mut Cache, limit: L) -> Result<Counter, Counter> {
        let count = self.claimed.incr(1);

        if cache.limit <= count {
            let recent_limit = limit.count();
            debug_assert!(recent_limit >= cache.limit);
            cache.limit = recent_limit;
        }

        if cache.limit <= count {
            Err(count)
        } else {
            Ok(count)
        }
    }

    fn commit(&self, _cache: &mut Cache, count: Counter) {
        let next = count + 1;

        while let Err(_) = self.count.cond_swap(count, next) {
            // spin
        }
    }
}
