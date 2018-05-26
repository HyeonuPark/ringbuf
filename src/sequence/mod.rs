
use std::fmt;

use counter::Counter;

pub trait Sequence: Default {
    type Cache: fmt::Debug;

    fn cache<L: Limit>(&self, limit: &L) -> Self::Cache;
    fn count(&self) -> Counter;

    fn claim<L: Limit>(&self, cache: &mut Self::Cache, limit: &L) -> Option<Counter>;
    fn commit(&self, cache: &mut Self::Cache, count: Counter);
}

/// For types `T: Sequence + !Shared`, calling `Sequence::cache()` more than once can panic.
pub trait Shared: Sequence {}

pub trait Limit {
    fn count(&self) -> Counter;
}
