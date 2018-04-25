
use counter::Counter;

pub mod owned;
pub mod shared;

pub trait Sequence: Default {
    type Cache: ::std::fmt::Debug;

    fn count(&self) -> Counter;
    fn cache<'a, L: Limit<'a>>(&self, limit: L) -> Self::Cache;
    fn try_claim<'a, L: Limit<'a>>(&self, cache: &mut Self::Cache, limit: L) -> Option<Counter>;
    fn claim<'a, L: Limit<'a>>(&self, cache: &mut Self::Cache, limit: L) -> Result<Counter, Counter>;
    fn commit(&self, cache: &mut Self::Cache, count: Counter);
}

pub trait Limit<'a> {
    fn count(&self) -> Counter;
}
