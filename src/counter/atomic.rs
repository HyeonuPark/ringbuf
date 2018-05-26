
use std::sync::atomic::{AtomicUsize, Ordering};
use std::fmt;

use super::Counter;

#[derive(Default)]
#[repr(align(64))]
pub struct AtomicCounter(AtomicUsize);

const LSB: usize = 1;

fn make(value: usize) -> Option<Counter> {
    if value & LSB == 0 {
        Some(Counter(value))
    } else {
        None
    }
}

impl AtomicCounter {
    /// Create new `AtomicCounter` from zero.
    pub fn new(value: Counter) -> Self {
        AtomicCounter(AtomicUsize::new(value.0))
    }

    /// Fetch internal counter. Returns `None` if closed.
    pub fn fetch(&self) -> Option<Counter> {
        make(self.0.load(Ordering::Acquire))
    }

    /// Increase internal counter. Returns previous counter or `None` if closed.
    pub fn incr(&self) -> Option<Counter> {
        make(self.0.fetch_add(2, Ordering::Release))
    }

    /// Conditionally change internal counter with given ordering.
    ///
    /// If internal counter is equal to `cond`, change it to `value` and returns `Ok(())`.
    /// If not, doesn't touch it and returns previous counter or `None` if closed.
    pub fn comp_swap(
        &self, cond: Counter, value: Counter, ord: Ordering
    ) -> Result<(), Option<Counter>> {
        let res = self.0.compare_and_swap(cond.0, value.0, ord);

        if res == cond.0 {
            Ok(())
        } else {
            Err(make(res))
        }
    }

    /// Close internal counter.
    ///
    /// Once closed, every operations of this `AtomicCounter` will returns `None`.
    pub fn close(&self) {
        let mut value = self.0.load(Ordering::Acquire);

        loop {
            // already closed
            if value & LSB != 0 {
                return
            }

            let prev = self.0.compare_and_swap(value, value | LSB, Ordering::Release);

            if prev == value {
                return
            } else {
                value = prev;
            }
        }
    }
}

impl From<Counter> for AtomicCounter {
    fn from(v: Counter) -> Self {
        AtomicCounter::new(v)
    }
}

impl From<usize> for AtomicCounter {
    fn from(v: usize) -> Self {
        AtomicCounter::new(v.into())
    }
}

impl fmt::Debug for AtomicCounter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.fetch() {
            None => f.write_str("AtomicCounter(Closed)"),
            Some(count) => write!(f, "AtomicCounter({})", count.0 >> 1),
        }
    }
}
