
use std::fmt;

use counter::Counter;

pub mod owned;

#[derive(Debug)]
pub struct Closed;

pub trait Sequence: Default {
    type Cache: fmt::Debug;

    fn cache<L: Limit>(&self, limit: &L) -> Result<Self::Cache, Closed>;
    fn count(&self) -> Result<Counter, Closed>;

    fn claim<L: Limit>(&self, cache: &mut Self::Cache, limit: &L) -> Option<Counter>;
    fn commit(&self, cache: &mut Self::Cache, count: Counter) -> Result<(), Closed>;
}

/// For types `T: Sequence + !Shared`, calling `Sequence::cache()` more than once can panic.
pub trait Shared: Sequence {}

pub trait Limit {
    fn count(&self) -> Counter;
}
