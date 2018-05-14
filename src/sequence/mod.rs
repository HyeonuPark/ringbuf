
use std::fmt::Debug;

use counter::Counter;

pub mod owned;
pub mod competitive;

pub use self::owned::Owned;
pub use self::competitive::Competitive;

pub trait Sequence: Sized + Default {
    type Cache: Debug;

    fn cache<L: Limit>(&self, limit: &L) -> Self::Cache;
    fn count(&self) -> Counter;

    fn try_claim<L: Limit>(&self, cache: &mut Self::Cache, limit: &L) -> Option<Counter>;
    fn commit(&self, cache: &mut Self::Cache, count: Counter);
}

pub trait Limit {
    fn count(&self) -> Counter;
}

pub unsafe trait Shared: Sequence {}
