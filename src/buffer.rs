
use std::sync::Arc;

use counter::Counter;

#[derive(Debug)]
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
    fn range(&self) -> (Counter, Counter);
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

impl<H: BufInfo, T> Clone for Buffer<H, T> {
    fn clone(&self) -> Self {
        Buffer {
            shared: self.shared.clone(),
            ptr: self.ptr,
            mask: self.mask,
        }
    }
}
