
use std::cmp::{self, PartialOrd};
use std::ops;
use std::fmt;

/// Overflow-safe ever-increasing pointer-sized counter.
///
/// This module provides types for incremental counter without upper limit, with closing detection.
/// To achieve this goal, `Counter` reserves 3 bits from its internal representation
/// so actual value space of it is `2^(sizeof usize - 3)`.
/// Comparing two `Counter`s whose difference is greater than this size may produce INVALID RESULT.
/// This is not checked by this type itself, so user MUST ensure that differences of counters
/// within same context never reach this level.
///
/// # Overflow-safety
///
/// `Counter` uses 2 MSB(Most Significant Bit)s for dealing with overflow.
/// This type has identical ordering semantics as `usize` with one exception:
/// value with MSB `0b00` is always considered greater than one with MSB `0b11`.
/// This makes ordering circular, and calculating relative difference of two counters is safe
/// if the invariant is guaranteed.
///
/// # Closing detection
///
/// `Counter` uses 1 LSB(Least Significant Bit) for dealing with closing.
/// Counter with LSB `0b1` is considered "closed" and other bits are ignored,
/// like `None` from `Option` type.
/// Every operations except `AtomicCounter::close` do not touch this flag.
/// 
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct Counter(pub(super) usize);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CounterRange {
    pub start: Counter,
    pub end: Counter,
}

pub const COUNTER_VALID_RANGE: usize = 1 << (WORD - 3);
const COUNTER_FULL_RANGE: isize = 1 << (WORD - 1);

const WORD: usize = ::std::mem::size_of::<usize>() * 8;
const MSB: usize = 0b11 << (WORD - 2);

fn msb_pp(value: Counter) -> bool {
    value.0 & MSB == MSB
}

fn msb_nn(value: Counter) -> bool {
    value.0 & MSB == 0
}

#[cfg(test)]
#[test]
fn test_counter_utils() {
    assert!(msb_pp(Counter(!0)));
    assert!(!msb_nn(Counter(!0)));
    assert!(!msb_pp(Counter(0)));
    assert!(msb_nn(Counter(0)));

    assert_ne!(COUNTER_VALID_RANGE << 2, 0);
    assert_eq!(COUNTER_VALID_RANGE << 3, 0);
}

impl Counter {
    /// Create new counter initialized with given value.
    pub fn new(init: usize) -> Self {
        Counter(init << 1)
    }

    /// Create a range of counters for iteration.
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
        let this = (self.0 >> 1) as isize;
        let that = (rhs.0 >> 1) as isize;

        if (msb_nn(self) && msb_pp(rhs)) || (msb_pp(self) && msb_nn(rhs)) {
            this.wrapping_sub(that).wrapping_add(COUNTER_FULL_RANGE)
        } else {
            this - that
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

impl fmt::Debug for Counter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Counter")
            .field(&(self.0 >> 1))
            .finish()
    }
}
