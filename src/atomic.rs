//! Atomically updatable thread-safe `Option<Arc<T>>`.

use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ops::Drop;
use std::mem;
use std::ptr::{self, NonNull};

#[derive(Debug)]
#[repr(align(64))]
pub struct Atomic<T> {
    ptr: AtomicPtr<T>,
}

#[derive(Debug)]
pub enum State<T> {
    /// State is not modified since initialization.
    /// `Atomic<T>` can't be changed once it modified to state other than `Empty`.
    Empty,

    /// State is closed and locked.
    /// It will not be changed during the lifetime of `Atomic<T>`.
    Closed,

    /// State contains an `Arc<T>`.
    /// It will not be changed during the lifetime of `Atomic<T>`.
    Holds(Arc<T>),
}

impl<T> Atomic<T> {
    /// Create an empty `Atomic<T>`.
    pub fn new() -> Self {
        Atomic {
            ptr: AtomicPtr::new(ptr::null_mut()),
        }
    }

    fn check_next(inner: *mut T) -> State<T> {
        match NonNull::new(inner) {
            None => State::Empty,
            Some(inner) if inner == NonNull::dangling() => State::Closed,
            Some(inner) => {
                let arc = unsafe { Arc::from_raw(inner.as_ptr()) };

                // let `Atomic` holds its `Arc`
                mem::forget(arc.clone());

                State::Holds(arc)
            }
        }
    }

    /// Fetch current state of this atomic.
    pub fn fetch(&self) -> State<T> {
        let inner = self.ptr.load(Ordering::Acquire);
        Self::check_next(inner)
    }

    /// Initialize this atomic if empty. Returns previous state.
    pub fn init(&self, value: Arc<T>) -> State<T> {
        let value = Arc::into_raw(value) as *mut T;
        let inner = self.ptr.compare_and_swap(ptr::null_mut(), value, Ordering::AcqRel);

        let res = Self::check_next(inner);

        match res {
            State::Empty => {}
            _ => unsafe {
                // `value` is not consumed
                // so drop it
                let _ = Arc::from_raw(value);
            }
        }

        res
    }

    /// Close this atomic if empty. Returns previous state.
    pub fn close(&self) -> State<T> {
        let dangling = NonNull::dangling().as_ptr();
        let inner = self.ptr.compare_and_swap(ptr::null_mut(), dangling, Ordering::AcqRel);

        Self::check_next(inner)
    }
}

impl<T> Drop for Atomic<T> {
    fn drop(&mut self) {
        let inner = *self.ptr.get_mut();

        // drop inner `Arc` if exist
        if let Some(inner) = NonNull::new(inner) {
            if inner != NonNull::dangling() {
                let _ = unsafe { Arc::from_raw(inner.as_ptr()) };
            }
        }
    }
}
