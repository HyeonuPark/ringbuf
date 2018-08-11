
use std::fmt;

use counter::{Counter, AtomicCounter};

pub mod owned;
pub mod shared;

pub trait Sequence: Default {
    type Cache: fmt::Debug;

    fn cache<L: Limit>(&self, limit: &L) -> Result<Self::Cache, CacheError>;
    fn counter(&self) -> &AtomicCounter;

    fn claim<L: Limit>(&self, cache: &mut Self::Cache, limit: &L) -> Option<Counter>;
    fn commit(&self, cache: &mut Self::Cache, count: Counter) -> Result<(), CommitError>;

    fn fetch_last(&self) -> Counter {
        match self.counter().fetch() {
            Ok(count) => count,
            Err(count) => count,
        }
    }
}

#[derive(Debug)]
pub enum CacheError {
    SeqClosed,
    NotAvailable,
}

#[derive(Debug)]
pub struct CommitError;

/// Sequences that can have more than one caches at the same time.
pub trait MultiCache: Sequence {}

pub trait Limit {
    fn count(&self) -> Counter;
}
