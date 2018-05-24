
use std::sync::Arc;
use std::ops::Drop;
use std::ptr;
use std::cmp::PartialEq;
use std::fmt;

use counter::{Counter, CounterRange, COUNTER_VALID_RANGE};

pub trait BufRange {
    fn range(&self) -> CounterRange;
}

pub struct Buffer<H: BufRange, T> {
    inner: Arc<Inner<H, T>>,
    ptr: *mut T,
    mask: usize,
}

#[derive(Debug)]
struct Inner<H: BufRange, T> {
    head: H,
    storage: Vec<T>,
}

unsafe impl<H: BufRange, T: Send> Send for Buffer<H, T> {}
unsafe impl<H: BufRange, T: Send> Sync for Buffer<H, T> {}

impl<H: BufRange, T> Buffer<H, T> {
    pub fn new(head: H, capacity: usize) -> Self {
        assert!(capacity.is_power_of_two(), "Capacity should be power of 2");
        assert!(capacity < COUNTER_VALID_RANGE,
            "Capacity should be lower than the maximum range of `Counter` type");

        let mut storage = Vec::with_capacity(capacity);
        let ptr = storage.as_mut_ptr();
        let mask = capacity - 1;

        let inner = Arc::new(Inner {
            head,
            storage,
        });

        Buffer {
            inner,
            ptr,
            mask,
        }
    }

    pub fn capacity(&self) -> usize {
        self.mask + 1
    }

    pub fn head(&self) -> &H {
        &self.inner.head
    }

    pub fn get(&self, count: Counter) -> *mut T {
        let index = (count & self.mask) as isize;
        unsafe {
            self.ptr.offset(index)
        }
    }
}

impl<H: BufRange, T> Drop for Inner<H, T> {
    fn drop(&mut self) {
        let elems = self.storage.as_mut_ptr();
        let mask = self.storage.capacity() - 1;

        for count in self.head.range() {
            let index = (count & mask) as isize;
            unsafe {
                ptr::drop_in_place(elems.offset(index));
            }
        }
    }
}

impl<H: BufRange, T> Clone for Buffer<H, T> {
    fn clone(&self) -> Self {
        Buffer {
            inner: self.inner.clone(),
            ptr: self.ptr,
            mask: self.mask,
        }
    }
}

impl<H: BufRange, T> PartialEq for Buffer<H, T> {
    fn eq(&self, rhs: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &rhs.inner)
    }
}

impl<H: BufRange + fmt::Debug, T: fmt::Debug> fmt::Debug for Buffer<H, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Buffer")
            .field("capacity", &self.capacity())
            .field("range", &self.head().range())
            .field("head", self.head())
            .field("contents", &PrintContents(self))
            .finish()
    }
}

struct PrintContents<'a, H: BufRange + fmt::Debug + 'a, T: fmt::Debug + 'a>(&'a Buffer<H, T>);

impl<'a, H: BufRange + fmt::Debug, T: fmt::Debug> fmt::Debug for PrintContents<'a, H, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list()
            .entries(self.0.head().range().map(|count| {
                let index = (count & self.0.mask) as isize;
                unsafe { &*self.0.ptr.offset(index) }
            }))
            .finish()
    }
}
