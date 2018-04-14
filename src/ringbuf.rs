use std::sync::Arc;
use std::ptr;

use counter::Counter;

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

impl<H, T> RingBuf<H, T> {
    /// # Panics
    ///
    /// It panics if capacity is not a power of 2.
    pub fn new(head: H, capacity: usize) -> Self {
        assert!(capacity.is_power_of_two(), "RingBuf's capacity MUST be a power of 2");

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

    pub fn capacity(&self) -> usize {
        self.mask + 1
    }

    pub fn head(&self) -> &H {
        &self.inner.head
    }

    pub unsafe fn get_ptr(&self, index: Counter) -> *mut T {
        let index = (index & self.mask) as isize;
        self.body_ptr.offset(index)
    }

    pub unsafe fn set(&self, index: Counter, value: T) {
        ptr::write(self.get_ptr(index), value);
    }

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
