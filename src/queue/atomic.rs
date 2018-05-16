
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
pub enum Next<T> {
    Empty,
    Closed,
    Holds(Arc<T>),
}

impl<T> Atomic<T> {
    pub fn new() -> Self {
        Atomic {
            ptr: AtomicPtr::new(ptr::null_mut()),
        }
    }

    fn check_next(inner: *mut T) -> Next<T> {
        match NonNull::new(inner) {
            None => Next::Empty,
            Some(inner) if inner == NonNull::dangling() => Next::Closed,
            Some(inner) => {
                let arc = unsafe { Arc::from_raw(inner.as_ptr()) };

                // let `Atomic` holds its `Arc`
                mem::forget(arc.clone());

                Next::Holds(arc)
            }
        }
    }

    pub fn fetch(&self) -> Next<T> {
        let inner = self.ptr.load(Ordering::Acquire);
        Self::check_next(inner)
    }

    pub fn init(&self, value: Arc<T>) -> Next<T> {
        let value = Arc::into_raw(value) as *mut T;
        let inner = self.ptr.compare_and_swap(ptr::null_mut(), value, Ordering::AcqRel);

        let res = Self::check_next(inner);

        match res {
            Next::Empty => {}
            _ => unsafe {
                // `value` is not consumed
                // so drop it
                let _ = Arc::from_raw(value);
            }
        }

        res
    }

    pub fn close(&self) -> Next<T> {
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
