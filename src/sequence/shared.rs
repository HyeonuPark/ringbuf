
use sequence::{Sequence, Limit};
use counter::{Counter, AtomicCounter};

/// Shared sequence.
///
/// This side of the channel can be accessed by multiple owners, from multiple threads.
#[derive(Debug, Default)]
pub struct Shared {
    count: AtomicCounter,
    claimed: AtomicCounter,
}

/// Cache for shared sequence.
///
/// Each owner of shared sequence has their own cache.
#[derive(Debug, Default, Clone, Copy)]
pub struct Cache {
    limit: Counter,
}

impl Sequence for Shared {
    type Cache = Cache;

    fn count(&self) -> Counter {
        self.count.fetch()
    }

    fn cache<L: Limit>(&self, limit: L) -> Cache {
        Cache {
            limit: limit.count(),
        }
    }

    fn claim<L: Limit>(&self, cache: &mut Cache, limit: L) -> Option<Counter> {
        let prev = self.claimed.incr(1);
        let next = prev + 1;

        if cache.limit <= prev {
            // try to push limit
            cache.limit = limit.count();
        }

        if cache.limit >= next {
            // claim succeeded
            Some(prev)
        } else {
            // claim failed
            loop {
                match self.claimed.cond_swap(next, prev) {
                    Ok(()) => return None,
                    Err(other_claimed) => {
                        // other sequence tried to claim
                        debug_assert!(other_claimed > next);

                        cache.limit = limit.count();
                        if cache.limit >= next {
                            // condition changed and now claim succeeded
                            return Some(prev);
                        }
                    }
                }
            }
        }
    }

    fn commit(&self, _cache: &mut Cache, index: Counter) {
        let prev = index;
        let next = index + 1;

        while self.count.cond_swap(prev, next).is_err() {
            // spin
        }
    }
}
