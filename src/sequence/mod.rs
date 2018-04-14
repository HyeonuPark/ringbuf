
mod owned;
mod shared;

pub use self::owned::{Owned, Cache as OwnedCache};
pub use self::shared::{Shared, Cache as SharedCache};

use counter::Counter;

pub trait Sequence: Sized + Default {
    type Cache: Copy + Default;

    fn count(&self) -> Counter;
    fn claim<L: Sequence>(&self, cache: &mut Self::Cache, limit: &L) -> Option<Counter>;
    fn commit(&self, cache: &mut Self::Cache, index: Counter);
}
