
use counter::Counter;

pub mod owned;
pub mod shared;

pub trait Sequence: Default {
    type Cache: ::std::fmt::Debug;

    fn count(&self) -> Counter;
    fn cache<L: Limit>(&self, limit: L) -> Self::Cache;
    fn try_claim<L: Limit>(&self, cache: &mut Self::Cache, limit: L) -> Option<Counter>;
    fn claim<L: Limit>(&self, cache: &mut Self::Cache, limit: L) -> Result<Counter, Counter>;
    fn commit(&self, cache: &mut Self::Cache, count: Counter);
}

pub trait Limit {
    fn count(&self) -> Counter;
}
