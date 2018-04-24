//! Sequence abstracts either end of the channel.
//!
//! Sender and receiver end of bounded channel have surprisingly similar behavior with each other.
//! Sender owns unoccupied half of buffer, while receiver owns occupied half. And they blocks
//! each other when the buffer is completely full or empty.

mod owned;
mod shared;

pub use self::owned::{Owned, Cache as OwnedCache};
pub use self::shared::{Shared, Cache as SharedCache};

use std::fmt::Debug;

use counter::Counter;

/// Abstraction over either end of the channel.
pub trait Sequence: Default + Sized {
    /// Sequence can own its state to cache shared atomic data, to improve performance.
    type Cache: Clone + Debug;

    /// Count that blocks opposite end of the channel.
    fn count(&self) -> Counter;

    /// Create local cache from sequence and limit.
    fn cache<L: Limit>(&self, limit: L) -> Self::Cache;

    /// Claim a new slot from this side of the channel. Returns `None` if failed.
    fn claim<L: Limit>(&self, cache: &mut Self::Cache, limit: L) -> Option<Counter>;

    /// Release claimed slot to the opposite end of the channel.
    fn commit(&self, cache: &mut Self::Cache, index: Counter);
}

/// View for the sequence at the opposite end.
pub trait Limit {
    /// Corresponds to `Sequence::count`.
    fn count(&self) -> Counter;
}
