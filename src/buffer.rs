
use std::sync::Arc;
use std::ptr;
use std::ops::Drop;
use std::cmp::PartialEq;
use std::fmt;

use counter::{Counter, CounterRange};

pub struct Buffer<H: BufInfo, T> {
    shared: Arc<Shared<H, T>>,
    ptr: *mut T,
    mask: usize,
}

#[derive(Debug)]
struct Shared<H: BufInfo, T> {
    head: H,
    body: Vec<T>,
}

pub trait BufInfo {
    fn range(&self) -> CounterRange;
}

unsafe impl<H: BufInfo + Sync, T: Send> Send for Buffer<H, T> {}
unsafe impl<H: BufInfo + Sync, T: Send> Sync for Buffer<H, T> {}

impl<H: BufInfo, T> Buffer<H, T> {
    pub fn new(head: H, capacity: usize) -> Self {
        assert!(capacity.is_power_of_two());
        assert_ne!(capacity << 2, 0);

        let mut body = Vec::with_capacity(capacity);
        let ptr = body.as_mut_ptr();

        let shared = Arc::new(Shared {
            head,
            body,
        });

        Buffer {
            shared,
            ptr,
            mask: capacity - 1,
        }
    }

    pub fn capacity(&self) -> usize {
        self.mask + 1
    }

    pub fn head(&self) -> &H {
        &self.shared.head
    }

    pub fn get_ptr(&self, count: Counter) -> *mut T {
        let index = (count & self.mask) as isize;
        unsafe { self.ptr.offset(index) }
    }
}

impl<H: BufInfo, T> Drop for Shared<H, T> {
    fn drop(&mut self) {
        let mask = self.body.capacity() - 1;
        let elems = self.body.as_mut_ptr();

        for count in self.head.range() {
            unsafe {
                let index = (count & mask) as isize;
                ptr::drop_in_place(elems.offset(index));
            }
        }
    }
}

impl<H: BufInfo, T> Clone for Buffer<H, T> {
    fn clone(&self) -> Self {
        Buffer {
            shared: self.shared.clone(),
            ptr: self.ptr,
            mask: self.mask,
        }
    }
}

impl<H: BufInfo, T> PartialEq for Buffer<H, T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.shared, &other.shared)
    }
}

struct PrintContents<'a, H: BufInfo + fmt::Debug + 'a, T: fmt::Debug + 'a>(&'a Buffer<H, T>);

impl<'a, H: BufInfo + fmt::Debug, T: fmt::Debug> fmt::Debug for PrintContents<'a, H, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list()
            .entries(self.0.head().range().map(|count| unsafe {
                self.0.get_ptr(count).as_ref().unwrap()
            }))
            .finish()

    }
}

impl<H: BufInfo + fmt::Debug, T: fmt::Debug> fmt::Debug for Buffer<H, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let range = self.head().range();

        f.debug_struct("Buffer")
            .field("capacity", &self.capacity())
            .field("start", &range.start)
            .field("end", &range.end)
            .field("contents", &PrintContents(self))
            .finish()
    }
}
