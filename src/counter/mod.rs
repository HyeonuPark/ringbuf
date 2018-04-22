//! Pointer-sized ever-increasing counters.
//!
//! This module provides types which represents incremental unbounded counters.
//! As their ever-increasing semantics within fixed-pointer-sized representation,
//! counters can only be meaningful when paired with other counters
//! to calculate their relative offsets.
//!
//! These counters are basic building blocks of all other components.

use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering as O};
use std::isize;
use std::cmp::{PartialOrd, Ordering};
use std::ops;

/// Pointer-sized incremental counter with overflow handling.
///
/// It has identical memory layout as `usize`, but doesn't break its meaning when overflowed.
///
/// To archive this goal, `Counter` uses its MSB as an overflow-detection field.
/// Initially all counters have their MSB as `0` and they are flipped when they overflow.
/// So if it's known that this counter is greater than that one and their difference is
/// less than `isize::MAX`, we can always calculate their offset safely regardless
/// either or both ones are overflowed.
///
/// Note that it's user's responsibility to keep these restrictions.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counter(usize);

/// Thread safe shared container for `Counter`.
///
/// Most operations use `Relaxed` ordering internally.
#[derive(Debug, Default)]
pub struct AtomicCounter(AtomicUsize);

/// Constant initializer for AtomicCounter
pub const ATOMIC_COUNTER_INIT: AtomicCounter = AtomicCounter(ATOMIC_USIZE_INIT);

const MSB: usize = !(isize::MAX as usize);

impl Counter {
    /// Create a new counter from zero.
    pub fn new() -> Self {
        Counter(0)
    }

    /// Split this counter to its overflow flag and internal representation.
    pub fn split(self) -> (bool, usize) {
        (
            self.0 & MSB != 0,
            self.0 & !MSB,
        )
    }

    /// Flip its overflow flag.
    pub fn flip(self) -> Self {
        Counter(self.0 ^ MSB)
    }
}

impl PartialOrd for Counter {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let (self_flag, self_val) = self.split();
        let (other_flag, other_val) = other.split();

        if self_flag ^ other_flag {
            Some(Ordering::Greater)
        } else {
            PartialOrd::partial_cmp(&self_val, &other_val)
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
    type Output = usize;

    fn sub(self, rhs: Self) -> usize {
        let (left_flag, left_val) = self.split();
        let (right_flag, right_val) = rhs.split();

        if left_flag ^ right_flag {
            MSB + left_val - right_val
        } else {
            left_val - right_val
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
