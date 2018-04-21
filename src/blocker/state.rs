use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
pub struct AtomicState {
    inner: AtomicUsize,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(usize)]
pub enum State {
    Neutral = 0,
    Locked,
    BlockSender,
    BlockReceiver,
    WakeSender,
    WakeReceiver,
}

impl AtomicState {
    pub fn new() -> Self {
        AtomicState {
            inner: AtomicUsize::new(State::Neutral as usize),
        }
    }

    pub fn transit(&self, cond: State, next: State) -> State {
        let prev = self.inner.compare_and_swap(
            cond as usize, next as usize, Ordering::AcqRel);
        unsafe { ::std::mem::transmute::<usize, State>(prev) }
    }
}
