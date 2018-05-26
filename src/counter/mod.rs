
mod counter;
mod atomic;

pub use self::counter::{Counter, CounterRange, COUNTER_VALID_RANGE};
pub use self::atomic::AtomicCounter;

#[cfg(test)]
mod tests;
