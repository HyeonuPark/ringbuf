
use std::sync::atomic::{AtomicUsize, Ordering};
use std::mem::size_of;
use std::fmt;

use super::Counter;

const PADDING_LEN: usize = 64 - 2 * size_of::<AtomicUsize>();

pub struct AtomicCounter {
    counter: AtomicUsize,
    last: AtomicUsize,
    /// To prevent false sharing
    _padding: [u8; PADDING_LEN],
}

const LSB: usize = 1;

fn make(value: usize) -> Option<Counter> {
    if value & LSB == 0 {
        Some(Counter(value))
    } else {
        None
    }
}

impl AtomicCounter {
    /// Create new `AtomicCounter` from given counter.
    pub fn new(value: Counter) -> Self {
        AtomicCounter {
            counter: value.0.into(),
            // Initially invalid counter
            last: 1.into(),
            _padding: [0; PADDING_LEN],
        }
    }

    /// Fetch internal counter. If closed, returns `Err(Counter)` with last counter observed.
    pub fn fetch(&self) -> Result<Counter, Counter> {
        make(self.counter.load(Ordering::Acquire)).ok_or_else(|| loop {
            if let Some(last) = make(self.last.load(Ordering::Acquire)) {
                break last;
            }
        })
    }

    /// Increase internal counter by 1. Returns previous counter or `None` if closed.
    pub fn incr(&self) -> Option<Counter> {
        // It actually increase `0b10` as its LSB is reserved for close detection.
        make(self.counter.fetch_add(0b10, Ordering::Release))
    }

    /// Conditionally change internal counter with given ordering.
    ///
    /// If internal counter is equal to `cond`, change it to `value` and returns `Ok(())`.
    /// If not, doesn't touch it and returns previous counter or `None` if closed.
    pub fn comp_swap(
        &self, cond: Counter, value: Counter, ord: Ordering
    ) -> Result<(), Option<Counter>> {
        let res = self.counter.compare_and_swap(cond.0, value.0, ord);

        if res == cond.0 {
            Ok(())
        } else {
            Err(make(res))
        }
    }

    /// Close internal counter.
    ///
    /// Once closed, every operations of this `AtomicCounter` should fail.
    pub fn close(&self) {
        let mut value = self.counter.load(Ordering::Acquire);

        loop {
            // already closed
            if value & LSB != 0 {
                return;
            }

            let prev = self.counter.compare_and_swap(value, LSB, Ordering::Release);

            if prev == value {
                break;
            } else {
                value = prev;
            }
        }

        // store last observed value
        self.last.store(value, Ordering::Release);
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

impl Default for AtomicCounter {
    fn default() -> Self {
        AtomicCounter::new(Counter::default())
    }
}

impl fmt::Debug for AtomicCounter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.fetch() {
            Ok(count) => write!(f, "AtomicCounter({})", count.0 >> 1),
            Err(last) => write!(f, "AtomicCounter(Closed, {})", last.0 >> 1),
        }
    }
}
