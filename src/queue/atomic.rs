use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr::{self, NonNull};
use std::mem;
use std::fmt;

const PADDING_SIZE: usize = 64 - mem::size_of::<AtomicPtr<()>>();

pub struct AtomicArc<T> {
    ptr: AtomicPtr<T>,
    _padding: [u8; PADDING_SIZE],
}

#[derive(Debug)]
pub enum AtomicState<T> {
    Empty,
    Closed,
    Holds(Arc<T>),
}

fn get_state<T>(raw: *mut T) -> AtomicState<T> {
    match NonNull::new(raw) {
        None => AtomicState::Empty,
        Some(non_null) => {
            if non_null == NonNull::dangling() {
                AtomicState::Closed
            } else {
                let arc = unsafe { Arc::from_raw(non_null.as_ptr()) };
                mem::forget(arc.clone());
                AtomicState::Holds(arc)
            }
        }
    }
}

impl<T> AtomicArc<T> {
    pub fn new() -> Self {
        AtomicArc {
            ptr: AtomicPtr::new(ptr::null_mut()),
            _padding: [0; PADDING_SIZE],
        }
    }

    /// Get current state.
    pub fn fetch(&self) -> AtomicState<T> {
        let inner = self.ptr.load(Ordering::Acquire);
        get_state(inner)
    }

    /// Initialize with given value if empty. Returns previous state.
    pub fn init(&self, value: Arc<T>) -> AtomicState<T> {
        let value = Arc::into_raw(value) as *mut T;
        let prev = self.ptr.compare_and_swap(ptr::null_mut(), value, Ordering::AcqRel);
        let state = get_state(prev);

        match state {
            AtomicState::Empty => {}
            AtomicState::Closed | AtomicState::Holds(_) => {
                // value is not consumed so drop it
                drop(unsafe {
                    Arc::from_raw(value)
                });
            }
        }

        state
    }

    /// Close if empty. Returns previous state.
    pub fn close(&self) -> AtomicState<T> {
        let dangling = NonNull::dangling().as_ptr();
        let prev = self.ptr.compare_and_swap(ptr::null_mut(), dangling, Ordering::AcqRel);
        get_state(prev)
    }
}

impl<T> Drop for AtomicArc<T> {
    fn drop(&mut self) {
        match self.fetch() {
            AtomicState::Empty | AtomicState::Closed => {}
            AtomicState::Holds(arc) => unsafe {
                // DOUBLE FREE!!!
                // Actually, release the ownership that `AtomicArc` holds.
                let raw = Arc::into_raw(arc);
                let _ = Arc::from_raw(raw);
                let _ = Arc::from_raw(raw);
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for AtomicArc<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.fetch(), f)
    }
}
