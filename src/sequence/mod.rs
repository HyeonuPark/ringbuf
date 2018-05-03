
use std::fmt::Debug;

use counter::Counter;

pub mod owned;
pub mod competitive;

pub trait Sequence: Sized {
    type Cache: Debug;

    fn new() -> (Self, Self::Cache);
    fn count(&self) -> Counter;

    fn try_claim<L: Limit>(&self, cache: &mut Self::Cache, limit: &L) -> Option<Counter>;
    fn commit(&self, cache: &mut Self::Cache, count: Counter);
}

pub trait Limit {
    fn limit(&self) -> Counter;
}

pub trait Shared: Sequence {
    fn new_cache<L: Limit>(&self, limit: &L) -> Self::Cache;
}
