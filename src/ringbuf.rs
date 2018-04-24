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
use std::ops::Drop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::cell::Cell;

use counter::Counter;

/// Shared circular ring buffer structure.
#[derive(Debug)]
pub struct RingBuf<H: BufInfo, T: Send> {
    mask: usize,
    inner: Arc<Inner<H, T>>,
    body_ptr: *mut T,
    is_closed: Cell<bool>,
}

#[derive(Debug)]
struct Inner<H, T> {
    head: H,
    body: Vec<T>,
    is_closed: AtomicBool,
}

/// `Head` should track occupied position of its buffer
/// to properly drop unsent messages.
pub trait BufInfo {
    /// Inclusive start position of this buffer.
    fn start(&self) -> Counter;

    /// Exclusive end position of this buffer.
    fn end(&self) -> Counter;
}

unsafe impl<H: BufInfo + Send, T: Send> Send for RingBuf<H, T> {}
unsafe impl<H: BufInfo + Sync, T: Send> Sync for RingBuf<H, T> {}

impl<H: BufInfo, T: Send> RingBuf<H, T> {
    /// # Panics
    ///
    /// It panics if capacity is not a power of 2, or its MSB is setted.
    pub fn new(head: H, capacity: usize) -> Self {
        assert!(capacity.is_power_of_two(), "Capacity MUST be a power of 2");
        assert!(capacity << 1 != 0, "capacity MUST NOT have its MSB setted");

        let mut body = Vec::with_capacity(capacity);
        let body_ptr = body.as_mut_ptr();

        let inner = Arc::new(Inner {
            head,
            body,
            is_closed: false.into(),
        });

        RingBuf {
            mask: capacity - 1,
            inner,
            body_ptr,
            is_closed: false.into(),
        }
    }

    /// Size of this buffer.
    pub fn capacity(&self) -> usize {
        self.mask + 1
    }

    /// Check if this buffer is closed.
    pub fn is_closed(&self) -> bool {
        if self.is_closed.get() {
            return true;
        }

        if self.inner.is_closed.load(Ordering::Acquire) {
            self.is_closed.set(true);
            true
        } else {
            false
        }
    }

    /// Close this buffer.
    pub fn close(&self) {
        if !self.is_closed.get() {
            self.is_closed.set(true);
            self.inner.is_closed.store(true, Ordering::Release);
        }
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

    /// Write given value to buffer's slot.
    pub unsafe fn write(&self, index: Counter, value: T) {
        ptr::write(self.get_ptr(index), value);
    }

    /// Read value from buffer's slot.
    pub unsafe fn read(&self, index: Counter) -> T {
        ptr::read(self.get_ptr(index))
    }
}

impl<H: BufInfo, T: Send> Clone for RingBuf<H, T> {
    fn clone(&self) -> Self {
        RingBuf {
            mask: self.mask,
            inner: self.inner.clone(),
            body_ptr: self.body_ptr,
            is_closed: self.is_closed.clone(),
        }
    }
}

impl<H: BufInfo, T: Send> Drop for RingBuf<H, T> {
    fn drop(&mut self) {
        let mut start = self.head().start();
        let end = self.head().end();

        while end > start {
            unsafe {
                self.read(start);
            }
            start += 1;
        }
    }
}
