use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering::Relaxed};
use std::ptr;
use std::num::Wrapping;

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

    pub unsafe fn get_ptr(&self, index: usize) -> *mut T {
        let index = (index & self.mask) as isize;
        self.body_ptr.offset(index)
    }

    pub unsafe fn set(&self, index: usize, value: T) {
        ptr::write(self.get_ptr(index), value);
    }

    pub unsafe fn take(&self, index: usize) -> T {
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

#[derive(Debug)]
pub struct Counter(AtomicUsize);

impl Counter {
    pub fn new() -> Self {
        Counter(AtomicUsize::new(0))
    }

    pub fn get(&self) -> usize {
        self.0.load(Relaxed)
    }

    pub fn incr(&self) -> usize {
        self.0.fetch_add(1, Relaxed)
    }
}

#[derive(Debug)]
pub struct Flag(AtomicBool);

impl Flag {
    pub fn new(init: bool) -> Self {
        Flag(AtomicBool::new(init))
    }

    pub fn get(&self) -> bool {
        self.0.load(Relaxed)
    }
}

const MSB: usize = !(::std::isize::MAX as usize);

fn split_msb(num: Wrapping<usize>) -> (bool, usize) {
    (
        num.0 & MSB != 0,
        num.0 & !MSB,
    )
}

pub fn check_gte(left: Wrapping<usize>, right: Wrapping<usize>) -> (usize, usize) {
    let (left_overflowed, mut left) = split_msb(left);
    let (right_overflowed, right) = split_msb(right);

    if left_overflowed ^ right_overflowed {
        left += MSB;
    }

    (left, right)
}
