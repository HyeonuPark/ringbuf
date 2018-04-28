
use counter::{Counter, AtomicCounter};
use sequence::{Sequence, Limit, Shared};

#[derive(Debug)]
pub struct Preemptive {
    count: AtomicCounter,
    claimed: AtomicCounter,
}

#[derive(Debug)]
pub struct Cache {
    limit: Counter,
}

impl Sequence for Preemptive {
    type Cache = Cache;

    fn and_cache() -> (Self, Cache) {
        (
            Preemptive {
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

    fn try_claim<L: Limit>(&self, cache: &mut Cache, limit: L) -> Option<Counter> {
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

    fn claim<L: Limit>(&self, cache: &mut Cache, limit: L) -> Result<Counter, Counter> {
        let claimed = self.claimed.incr(1);

        if cache.limit <= claimed {
            let recent_limit = limit.count();
            debug_assert!(recent_limit >= cache.limit);
            cache.limit = recent_limit;
        }

        if cache.limit <= claimed {
            Err(claimed)
        } else {
            Ok(claimed)
        }
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

impl Shared for Preemptive {
    fn new_cache<L: Limit>(&self, limit: L) -> Cache {
        Cache {
            limit: limit.count(),
        }
    }
}
