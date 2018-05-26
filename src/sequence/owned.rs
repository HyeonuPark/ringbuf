
use counter::{Counter, AtomicCounter};
use sequence::{Sequence, Limit, Closed};

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

    fn cache<L: Limit>(&self, limit: &L) -> Result<Cache, Closed> {
        Ok(Cache {
            count: self.count()?,
            limit: limit.count(),
        })
    }

    fn count(&self) -> Result<Counter, Closed> {
        self.count.fetch().ok_or(Closed)
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

    fn commit(&self, cache: &mut Cache, count: Counter) -> Result<(), Closed> {
        match self.count.incr() {
            None => Err(Closed),
            Some(prev) => {
                debug_assert_eq!(prev, count);
                debug_assert_eq!(cache.count, prev + 1);
                Ok(())
            }
        }
    }
}
