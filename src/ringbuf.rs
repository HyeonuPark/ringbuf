//! Fixed-sized ring buffer that can be shared between threads.
//!
//! `RingBuf` cosists of two parts: Head and Body.
//! Head is simple struct that can safely accessed from multiple threads.
//! It's intended to be a synchronization point between producers and consumers.
//!
//! Body is a fixed size array that can be indexed by [`Counter`](../counter/struct.Counter.html).
//! Actual offset is calculated using modular arithmetic.
//!
//! Note that [`RingBuf`](RingBuf) is very low-level structure and it's just unsafe as
//! raw pointer.

use std::sync::Arc;
use std::ptr;

use counter::Counter;

/// Circular ring buffer structure.
#[derive(Debug)]
pub struct RingBuf<H, T> {
    mask: usize,
    inner: Arc<Inner<H, T>>,
    body_ptr: *mut T,
}

#[derive(Debug)]
struct Inner<H, T> {
    head: H,
    body: Vec<T>,
}

unsafe impl<H: Send, T: Send> Send for RingBuf<H, T> {}
unsafe impl<H: Sync, T: Send> Sync for RingBuf<H, T> {}

impl<H, T> RingBuf<H, T> {
    /// # Panics
    ///
    /// It panics if capacity is not a power of 2, or its MSB is setted.
    pub fn new(head: H, capacity: usize) -> Self {
        use std::isize;
        assert!(capacity.is_power_of_two(), "Capacity MUST be a power of 2");
        assert!(capacity & !(isize::MAX as usize) == 0, "capacity MUST NOT have its MSB setted");

        let mut body = Vec::with_capacity(capacity);
        let body_ptr = body.as_mut_ptr();

        let inner = Arc::new(Inner {
            head,
            body,
        });

        RingBuf {
            mask: capacity - 1,
            inner,
            body_ptr,
        }
    }

    /// Size of this buffer.
    pub fn capacity(&self) -> usize {
        self.mask + 1
    }

    /// Access buffer's head.
    pub fn head(&self) -> &H {
        &self.inner.head
    }

    /// Get pointer of buffer's slot from counter.
    pub unsafe fn get_ptr(&self, index: Counter) -> *mut T {
        let index = (index & self.mask) as isize;
        self.body_ptr.offset(index)
    }

    /// Set buffer's slot with given value.
    pub unsafe fn set(&self, index: Counter, value: T) {
        ptr::write(self.get_ptr(index), value);
    }

    /// Read out value from buffer's slot.
    pub unsafe fn take(&self, index: Counter) -> T {
        ptr::read(self.get_ptr(index))
    }
}

impl<H, T> Clone for RingBuf<H, T> {
    fn clone(&self) -> Self {
        RingBuf {
            mask: self.mask,
            inner: self.inner.clone(),
            body_ptr: self.body_ptr,
        }
    }
}
