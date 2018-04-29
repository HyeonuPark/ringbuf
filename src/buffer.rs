
use std::sync::Arc;
use std::ops::{Index, Drop};
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::{mem, ptr};

#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

use counter::Counter;
use blocker::{Blocker, BlockerNode, BlockerContainer};

#[derive(Debug)]
pub struct Buffer<H: BufInfo, T> {
    inner: Arc<Inner<H, T>>,
    ptr: *const Bucket<T>,
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
    body: Vec<Bucket<T>>,
}

#[derive(Debug)]
pub struct Bucket<T> {
    _marker: PhantomData<T>,
    blockers: BlockerContainer,
    inner: UnsafeCell<T>,
    #[cfg(debug_assertions)]
    has_inner: AtomicBool,
}

impl<H: BufInfo, T: Send> Buffer<H, T> {
    pub fn new(head: H, capacity: usize) -> Self {
        assert!(capacity.is_power_of_two());
        assert_ne!(capacity << 2, 0);

        let mut body: Vec<_> = (0..capacity).map(|_| Bucket::new()).collect();
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

    pub fn get(&self, count: Counter) -> &Bucket<T> {
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
    type Output = Bucket<T>;

    fn index(&self, index: Counter) -> &Bucket<T> {
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

impl<T> Bucket<T> {
    fn new() -> Self {
        Bucket {
            _marker: PhantomData,
            blockers: Blocker::container(),
            inner: UnsafeCell::new(unsafe { mem::uninitialized() }),
            #[cfg(debug_assertions)]
            has_inner: false.into(),
        }
    }

    pub fn get(&self) -> T {
        #[cfg(debug_assertions)]
        assert!(self.has_inner.load(SeqCst),
            "Bucket::get should not be called on empty slot");

        let res = unsafe {
            ptr::read(self.inner.get())
        };

        #[cfg(debug_assertions)]
        self.has_inner.store(false, SeqCst);

        res
    }

    pub fn set(&self, item: T) {
        #[cfg(debug_assertions)]
        assert!(!self.has_inner.load(SeqCst),
            "Bucket::set should not be called on non-empty Bucket");

        unsafe {
            ptr::write(self.inner.get(), item);
        }

        #[cfg(debug_assertions)]
        self.has_inner.store(true, SeqCst);
    }

    pub fn register(&self, blocker: BlockerNode) {
        self.blockers.push(blocker);
    }

    pub fn notify(&self) {
        if let Some(blocker) = self.blockers.pop() {
            blocker.unblock();
        }
    }
}
