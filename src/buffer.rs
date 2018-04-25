
use std::sync::Arc;
use std::ptr;
use std::ops::Drop;

use counter::Counter;

#[derive(Debug)]
pub struct Buffer<H: BufInfo, T: Send> {
    inner: Arc<Inner<H, T>>,
    ptr: *mut T,
    mask: usize,
}

#[derive(Debug)]
struct Inner<H: BufInfo, T: Send> {
    head: H,
    body: Vec<T>,
}

pub trait BufInfo {
    fn start(&self) -> Counter;
    fn end(&self) -> Counter;
}

unsafe impl<H: BufInfo, T: Send> Send for Buffer<H, T> {}

impl<H: BufInfo, T: Send> Buffer<H, T> {
    pub fn new(head: H, capacity: usize) -> Self {
        assert!(capacity.is_power_of_two(), "Capacity MUST be a power of 2");
        assert!(capacity << 1 != 0, "capacity MUST NOT have its MSB setted");

        let mut body = Vec::with_capacity(capacity);
        let ptr = body.as_mut_ptr();

        let inner = Arc::new(Inner {
            head,
            body,
        });

        Buffer {
            inner,
            ptr,
            mask: capacity - 1,
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
        self.ptr.offset(index)
    }

    pub unsafe fn read(&self, index: Counter) -> T {
        ptr::read(self.get_ptr(index))
    }

    pub unsafe fn write(&self, index: Counter, value: T) {
        ptr::write(self.get_ptr(index), value)
    }
}

impl<H: BufInfo, T: Send> Clone for Buffer<H, T> {
    fn clone(&self) -> Self {
        Buffer {
            inner: self.inner.clone(),
            ptr: self.ptr,
            mask: self.mask,
        }
    }
}

impl<H: BufInfo, T: Send> Drop for Inner<H, T> {
    fn drop(&mut self) {
        let ptr = self.body.as_mut_ptr();
        let (start, end) = (self.head.start(), self.head.end());
        let mut index = start;

        while end > index {
            unsafe {
                ptr::drop_in_place(ptr.offset((index - start) as isize));
            }
            index += 1;
        }
    }
}
