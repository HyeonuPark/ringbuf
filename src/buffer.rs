
use std::sync::Arc;
use std::ops::{Index, Drop};

use counter::Counter;

#[derive(Debug)]
pub struct Buffer<H: BufInfo, T> {
    inner: Arc<Inner<H, T>>,
    ptr: *const T,
    mask: usize,
}

pub trait BufInfo {
    fn start(&self) -> Counter;
    fn end(&self) -> Counter;
}

#[derive(Debug)]
#[repr(C)]
struct Inner<H: BufInfo, T> {
    head: H,
    body: Vec<T>,
}

impl<H: BufInfo, T: Default> Buffer<H, T> {
    pub fn new(head: H, capacity: usize) -> Self {
        assert!(capacity.is_power_of_two());
        assert_ne!(capacity << 1, 0);

        let mut body: Vec<T> = (0..capacity).map(|_| T::default()).collect();
        let ptr = body.as_ptr();

        unsafe { body.set_len(0); }

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
}

impl<H: BufInfo, T> Buffer<H, T> {
    pub fn capacity(&self) -> usize {
        self.mask + 1
    }

    pub fn head(&self) -> &H {
        &self.inner.head
    }

    pub fn get(&self, count: Counter) -> &T {
        let index = (count & self.mask) as isize;
        unsafe { &*self.ptr.offset(index) }
    }
}

unsafe impl<H: BufInfo, T: Sync> Send for Buffer<H, T> {}

impl<H: BufInfo, T> Clone for Buffer<H, T> {
    fn clone(&self) -> Self {
        Buffer {
            inner: self.inner.clone(),
            ptr: self.ptr,
            mask: self.mask,
        }
    }
}

impl<H: BufInfo, T> Index<Counter> for Buffer<H, T> {
    type Output = T;

    fn index(&self, index: Counter) -> &T {
        self.get(index)
    }
}

impl<H: BufInfo, T> Drop for Inner<H, T> {
    fn drop(&mut self) {
        use std::ptr;

        let mask = self.body.capacity() - 1;
        let start = self.head.start();
        let end = self.head.end();
        let mut index = start;

        while end > index {
            unsafe {
                let elem = self.body.get_unchecked_mut(index & mask);
                ptr::drop_in_place(elem);
            }
            index += 1;
        }
    }
}
