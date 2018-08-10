
use std::fmt;
use std::sync::atomic::AtomicUsize;

use counter::{Counter, AtomicCounter};

pub mod owned;
pub mod shared;

#[derive(Debug)]
pub struct Closed;

pub(crate) trait Sequence: Default {
    type Cache: fmt::Debug;

    fn cache<L: Limit>(&self, limit: &L) -> Option<Self::Cache>;
    fn counter(&self) -> &AtomicCounter;

    fn claim<L: Limit>(&self, cache: &mut Self::Cache, limit: &L) -> Option<Counter>;
    fn commit(&self, cache: &mut Self::Cache, count: Counter) -> Result<(), Closed>;
}

/// For types `T: Sequence + !MultiCache`, calling `Sequence::cache()` more than once can panic.
pub(crate) trait MultiCache: Sequence {
    fn cache_counter(&self) -> &AtomicUsize;
}

pub trait Limit {
    fn count(&self) -> Counter;
}
