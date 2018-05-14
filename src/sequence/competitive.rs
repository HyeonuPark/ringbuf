
use counter::{Counter, AtomicCounter};
use sequence::{Sequence, Limit, Shared};

#[derive(Debug, Default)]
pub struct Competitive {
    count: AtomicCounter,
    claimed: AtomicCounter,
}

#[derive(Debug)]
pub struct Cache {
    limit: Counter,
}

impl Sequence for Competitive {
    type Cache = Cache;

    unsafe fn cache_unchecked<L: Limit>(&self, limit: &L) -> Cache {
        Cache {
            limit: limit.count(),
        }
    }

    fn count(&self) -> Counter {
        self.count.fetch()
    }

    fn try_claim<L: Limit>(&self, cache: &mut Cache, limit: &L) -> Option<Counter> {
        let mut claim = self.claimed.fetch();

        loop {
            if claim >= cache.limit {
                let recent_limit = limit.count();
                debug_assert!(recent_limit >= cache.limit);
                cache.limit = recent_limit;
            }

            if claim >= cache.limit {
                return None;
            }

            match self.claimed.cond_swap(claim, claim + 1) {
                Ok(()) => return Some(claim),
                Err(prev) => claim = prev,
            }
        }
    }

    fn commit(&self, _cache: &mut Cache, count: Counter) {
        let next = count + 1;

        while let Err(_) = self.count.cond_swap(count, next) {
            // spin
        }
    }
}

unsafe impl Shared for Competitive {}
