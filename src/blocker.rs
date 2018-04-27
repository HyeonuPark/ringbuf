
use std::thread::Thread;
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr::{self, NonNull};
use std::cell::Cell;

use counter::{Counter, AtomicCounter};

#[derive(Default)]
pub struct Blocker {
    kind: Cell<Option<BlockKind>>,
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

    pub fn unblock(&self) {
        match self.kind.take().expect("Calling Blocker::unblock on non-blocked case") {
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

    pub fn push(&self, next: &mut Blocker) {
        next.stamp.incr(1); // To avoid ABA problem

        let mut prev = self.head.load(Ordering::Relaxed);
        let mut prev_stamp = fetch_stamp(prev);

        loop {
            next.next = AtomicPtr::new(prev);

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

    pub fn pop(&self) -> Option<NonNull<Blocker>> {
        loop {
            let prev_ptr = self.head.load(Ordering::Relaxed);

            match NonNull::new(prev_ptr) {
                None => return None,
                Some(prev) => {
                    let prev_stamp = fetch_stamp(prev_ptr);

                    let next = unsafe { &prev.as_ref().next };
                    let next = next.load(Ordering::Relaxed);

                    let swap = self.head.compare_and_swap(prev_ptr, next, Ordering::Relaxed);
                    let swap_stamp = fetch_stamp(swap);

                    if ptr::eq(prev_ptr, swap) && prev_stamp == swap_stamp {
                        return Some(prev);
                    }
                }
            }
        }
    }
}

fn fetch_stamp(blocker: *mut Blocker) -> Option<Counter> {
    unsafe {
        blocker.as_ref().map(|blocker| blocker.stamp.fetch())
    }
}
