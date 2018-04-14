//! Sequence abstracts either end of the channel.
//!
//! Sending and receiving end of bound channel have surprisingly similar behavior with each other.
//! Sender owns occupied half of buffer, while receiver owns unoccupied half. And they blocks
//! each other when the buffer is completely full or empty.

mod owned;
mod shared;

pub use self::owned::{Owned, Cache as OwnedCache};
pub use self::shared::{Shared, Cache as SharedCache};

use counter::Counter;

/// Abstraction over either end of the channel.
pub trait Sequence: Sized + Default {
    /// Sequence can own its state to cache shared atomic data, to improve performance.
    type Cache: Default;

    /// Count that blocks opposite end of the channel.
    fn count(&self) -> Counter;

    /// Claim a new slot from this side of the channel. Returns `None` if failed.
    fn claim<L: Sequence>(&self, cache: &mut Self::Cache, limit: &L) -> Option<Counter>;

    /// Release claimed slot to the opposite end of the channel.
    fn commit(&self, cache: &mut Self::Cache, index: Counter);
}
