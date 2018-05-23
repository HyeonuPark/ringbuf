//! Overflow-safe ever-increasing pointer-sized counter.
//!
//! This module provides types for incremental counter without upper limit, with closing detection.
//! To achieve this goal, `Counter` reserves 3 bits from its internal representation
//! so actual value space of it is `2^(sizeof usize - 3)`.
//! Comparing two `Counter`s whose difference is greater than this size may produce INVALID RESULT.
//! This is not checked by this type itself, so user MUST ensure that differences of counters
//! within same context never reach this level.
//!
//! # Overflow-safety
//!
//! `Counter` uses 2 MSB(Most Significant Bit)s for dealing with overflow.
//! This type has identical ordering semantics as `usize` with one exception:
//! value with MSB `0b00` is always considered greater than one with MSB `0b11`.
//! This makes ordering circular, and calculating relative difference of two counters is safe
//! if the invariant is guaranteed.
//!
//! # Closing detection
//!
//! `Counter` uses 1 LSB(Least Significant Bit) for dealing with closing.
//! Counter with LSB `0b1` is considered "closed" and other bits are ignored,
//! like `None` from `Option` type.
//! Every operations except `AtomicCounter::close` do not touch this flag.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::cmp::{self, PartialOrd};
use std::ops;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counter(usize);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CounterRange {
    pub start: Counter,
    pub end: Counter,
}

#[derive(Debug, Default)]
#[repr(align(64))]
pub struct AtomicCounter(AtomicUsize);

pub const COUNTER_VALID_RANGE: usize = 1 << (WORD - 3);

#[cfg(test)]
mod tests;

const WORD: usize = ::std::mem::size_of::<usize>();

const LSB: usize = 1;
const MSB: usize = 0b11 << (WORD - 2);

fn make(value: usize) -> Option<Counter> {
    if value & LSB == 0 {
        Some(Counter(value))
    } else {
        None
    }
}

fn msb_pp(value: Counter) -> bool {
    value.0 & MSB == MSB
}

fn msb_nn(value: Counter) -> bool {
    value.0 & MSB == 0
}

impl Counter {
    /// Create new counter initialized with given value.
    pub fn new(init: usize) -> Self {
        Counter(init << 1)
    }

    /// Create a range of counter for iteration.
    pub fn range(start: Counter, end: Counter) -> CounterRange {
        CounterRange {
            start,
            end,
        }
    }
}

impl Iterator for CounterRange {
    type Item = Counter;

    fn next(&mut self) -> Option<Counter> {
        if self.start == self.end {
            None
        } else {
            let res = self.start;
            self.start = res + 1;
            Some(res)
        }
    }
}

impl PartialOrd<Self> for Counter {
    fn partial_cmp(&self, rhs: &Self) -> Option<cmp::Ordering> {
        if msb_nn(*self) && msb_pp(*rhs) {
            Some(cmp::Ordering::Greater)
        } else if msb_pp(*self) && msb_nn(*rhs) {
            Some(cmp::Ordering::Less)
        } else {
            PartialOrd::partial_cmp(&self.0, &rhs.0)
        }
    }
}

impl ops::Sub<Self> for Counter {
    type Output = isize;

    fn sub(self, rhs: Self) -> isize {
        if msb_nn(self) && msb_pp(rhs) {
            rhs.0.wrapping_sub(self.0) as isize
        } else if msb_pp(self) && msb_nn(rhs) {
            - (rhs.0.wrapping_sub(self.0) as isize)
        } else {
            self.0.wrapping_sub(rhs.0) as isize
        }
    }
}

impl ops::Add<usize> for Counter {
    type Output = Self;

    fn add(self, rhs: usize) -> Self {
        Counter(self.0.wrapping_add(rhs << 1))
    }
}

impl ops::AddAssign<usize> for Counter {
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

impl ops::Sub<usize> for Counter {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self {
        Counter(self.0.wrapping_sub(rhs << 1))
    }
}

impl ops::SubAssign<usize> for Counter {
    fn sub_assign(&mut self, rhs: usize) {
        *self = *self - rhs;
    }
}

impl ops::BitAnd<usize> for Counter {
    type Output = usize;

    fn bitand(self, rhs: usize) -> usize {
        (self.0 >> 1) & rhs
    }
}

impl ops::BitOr<usize> for Counter {
    type Output = usize;

    fn bitor(self, rhs: usize) -> usize {
        (self.0 >> 1) | rhs
    }
}

impl From<usize> for Counter {
    fn from(v: usize) -> Self {
        Counter::new(v)
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
