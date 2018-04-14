
use std::sync::atomic::{AtomicUsize, Ordering as O};
use std::isize;
use std::cmp::{PartialOrd, Ordering};
use std::ops;

/// Pointer-sized incremental counter with overflow-handling
///
/// It's like Wrapping<usize> but uses its MSB as an overflow-detection field.
///
/// `SignWrap` has special functionality with comparing.
/// If only one of either operands have its MSB setted,
/// it assumes that the left-hand-side operand is overflowed,
/// so it works as LEFT > RIGHT.
///
/// This assumption will be false if two counter's difference exceeds `isize::MAX`.
/// So one MUST make sure it will not happen when using this counter.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counter(usize);

/// Shared atomic counter
#[derive(Debug, Default)]
pub struct AtomicCounter(AtomicUsize);

const MSB: usize = !(isize::MAX as usize);

impl Counter {
    pub fn new() -> Self {
        Counter(0)
    }

    pub fn split(self) -> (bool, usize) {
        (
            self.0 & MSB != 0,
            self.0 & !MSB,
        )
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
    pub fn new() -> Self {
        AtomicCounter(AtomicUsize::new(0))
    }

    pub fn get(&self) -> Counter {
        Counter(self.0.load(O::Relaxed))
    }

    pub fn incr(&self, amount: usize) -> Counter {
        Counter(self.0.fetch_add(amount, O::Relaxed))
    }

    pub fn cond_swap(&self, cond: Counter, swap: Counter) -> Result<(), Counter> {
        // QUESTION: is it possible to replace SeqCst with less restricted ordering?
        let prev_usize = self.0.compare_and_swap(cond.0, swap.0, O::SeqCst);

        if prev_usize == cond.0 {
            Ok(())
        } else {
            Err(Counter(prev_usize))
        }
    }
}
