
use std::thread::Thread;
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

use counter::{Counter, AtomicCounter};

#[derive(Default)]
pub struct Blocker {
    kind: Option<BlockKind>,
    stamp: AtomicCounter,
    next: AtomicPtr<Blocker>,
}

#[derive(Debug)]
pub enum BlockKind {
    Sync(Thread),
}

/// An intrusive treiber stack of `Blocker`s.
///
/// Blockers can be push/pop-ed atomically.
#[derive(Debug)]
pub struct BlockerStack {
    head: AtomicPtr<Blocker>,
}

impl Blocker {
    pub fn new() -> Arc<Self> {
        Arc::new(Blocker::default())
    }

    pub fn block(&mut self, kind: BlockKind) {
        self.kind = Some(kind);
    }

    pub fn unblock(&self) {
        let kind = self.kind.as_ref().expect("Calling Blocker::unblock on non-blocked case");
        match *kind {
            BlockKind::Sync(ref thread) => {
                thread.unpark();
            }
        }
    }
}

impl BlockerStack {
    pub fn new() -> Self {
        BlockerStack {
            head: AtomicPtr::new(ptr::null_mut()),
        }
    }

    pub fn push(&self, next: Arc<Blocker>) {
        next.stamp.incr(1); // To avoid ABA problem

        let next = Arc::into_raw(next) as *mut Blocker;
        let mut prev = self.head.load(Ordering::Relaxed);
        let mut prev_stamp = fetch_stamp(prev);

        loop {
            unsafe { &*next }.next.store(prev, Ordering::Relaxed);

            let swap = self.head.compare_and_swap(prev, next, Ordering::Relaxed);
            let swap_stamp = fetch_stamp(swap);

            if ptr::eq(prev, swap) && swap_stamp == prev_stamp {
                return;
            } else {
                prev = swap;
                prev_stamp = fetch_stamp(swap);
            }
        }
    }

    pub fn pop(&self) -> Option<Arc<Blocker>> {
        loop {
            let prev = self.head.load(Ordering::Relaxed);

            if prev.is_null() {
                return None;
            }

            let prev_stamp = fetch_stamp(prev);
            let next = unsafe { &*prev }.next.load(Ordering::Relaxed);

            let swap = self.head.compare_and_swap(prev, next, Ordering::Acquire);
            let swap_stamp = fetch_stamp(swap);

            if ptr::eq(prev, swap) && prev_stamp == swap_stamp {
                let prev = unsafe { Arc::from_raw(prev) };
                return Some(prev);
            }
        }
    }
}

fn fetch_stamp(blocker: *mut Blocker) -> Option<Counter> {
    unsafe {
        blocker.as_ref().map(|blocker| blocker.stamp.fetch())
    }
}
