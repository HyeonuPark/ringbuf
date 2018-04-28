//! Pointer-sized ever-increasing counters.
//!
//! This module provides types which represents incremental unbounded counters.
//! As their ever-increasing semantics within fixed-pointer-sized representation,
//! counters can only be meaningful when paired with other counters
//! to calculate their relative offsets.
//!
//! These counters are basic building blocks of all other components.

use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering as O};
use std::cmp::{PartialOrd, Ordering};
use std::ops;

/// Pointer-sized incremental counter with overflow handling.
///
/// It has identical memory layout as `usize`, but doesn't break its meaning when overflowed.
///
/// To archive this goal, `Counter` splits the value space of `usize` into 3 regions
/// based on their highest 2 bits. Basically, it follows same ordering semantics as `usize`,
/// but counters with highest bits `11` is considered ALWAYS LESS THAN ones with `00`.
///
/// This makes the ordering semantics of `Counter` circular, and comparing 2 counters
/// whose difference is greater than `usize::MAX >> 2` can produce INVALID RESULT.
/// So user MUST ensure that differences of counters within same context never reach this level.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counter(usize);

/// Thread safe shared container for `Counter`.
#[derive(Debug, Default)]
pub struct AtomicCounter(AtomicUsize);

/// Constant initializer for AtomicCounter
pub const ATOMIC_COUNTER_INIT: AtomicCounter = AtomicCounter(ATOMIC_USIZE_INIT);

const FLAG: usize = !(!0 >> 2);

impl Counter {
    /// Create a new counter.
    pub fn new(num: usize) -> Self {
        Counter(num)
    }
}

impl PartialOrd for Counter {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if FLAG & self.0 & !other.0 == FLAG {
            Some(Ordering::Less)
        } else if FLAG & !self.0 & other.0 == FLAG {
            Some(Ordering::Greater)
        } else {
            PartialOrd::partial_cmp(&self.0, &other.0)
        }
    }
}

impl ops::Add<usize> for Counter {
    type Output = Self;

    fn add(self, rhs: usize) -> Self {
        Counter(self.0.wrapping_add(rhs))
    }
}

impl ops::AddAssign<usize> for Counter {
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

impl ops::Sub<Self> for Counter {
    type Output = isize;

    fn sub(self, rhs: Self) -> isize {
        if FLAG & self.0 & !rhs.0 == FLAG {
            0isize - (rhs.0 as isize) - (!0 - self.0) as isize
        } else if FLAG & !self.0 & rhs.0 == FLAG {
            1isize + (self.0 as isize) + (!0 - rhs.0) as isize
        } else {
            unsafe {
                ::std::mem::transmute(self.0.wrapping_sub(rhs.0))
            }
        }
    }
}

impl ops::BitAnd<usize> for Counter {
    type Output = usize;

    fn bitand(self, rhs: usize) -> usize {
        self.0 & rhs
    }
}

impl ops::BitOr<usize> for Counter {
    type Output = usize;

    fn bitor(self, rhs: usize) -> usize {
        self.0 | rhs
    }
}

impl AtomicCounter {
    /// Create a new atomic counter from zero.
    pub fn new() -> Self {
        AtomicCounter(AtomicUsize::new(0))
    }

    /// Fetch counter to determine somebody changed it.
    ///
    /// It uses `Acquire` ordering.
    pub fn fetch(&self) -> Counter {
        Counter(self.0.load(O::Acquire))
    }

    /// Increase counter atomically and returns previous one.
    ///
    /// It uses `Release` ordering.
    pub fn incr(&self, amount: usize) -> Counter {
        Counter(self.0.fetch_add(amount, O::Release))
    }

    /// If its inner counter is same as `cond`, replace it with `value` and returns `Ok(())`.
    /// If not, left itself unchanged and returns copy of inner counter as `Err`.
    ///
    /// It uses `SeqCst` ordering with `compare_and_swap`.
    pub fn cond_swap(&self, cond: Counter, value: Counter) -> Result<(), Counter> {
        debug_assert!(value > cond, "Counter should be increase-only");

        let prev_usize = self.0.compare_and_swap(cond.0, value.0, O::SeqCst);

        if prev_usize == cond.0 {
            Ok(())
        } else {
            Err(Counter(prev_usize))
        }
    }
}

#[cfg(test)]
mod tests;
