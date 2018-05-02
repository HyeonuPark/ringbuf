
use std::sync::atomic::{AtomicUsize, Ordering};
use std::marker::PhantomData;
use std::cmp::PartialEq;

/// `AtomicPtr<T>` with ABA flag support
#[derive(Debug)]
#[repr(align(64))]
pub struct Atomic<T> {
    _marker: PhantomData<T>,
    inner: AtomicUsize,
}

/// `*mut T` with ABA flag support
///
/// T must be aligned with at least 64 bytes
#[derive(Debug, Eq)]
pub struct Ptr<T> {
    flag: usize,
    inner: *mut T,
}

const FLAG: usize = 0b111111;

impl<T> Atomic<T> {
    pub fn null() -> Self {
        Atomic {
            _marker: PhantomData,
            inner: AtomicUsize::new(0),
        }
    }

    pub fn new(ptr: Ptr<T>) -> Self {
        Atomic {
            _marker: PhantomData,
            inner: AtomicUsize::new(ptr.merge()),
        }
    }

    pub fn load(&self, ord: Ordering) -> Ptr<T> {
        Ptr::restore(self.inner.load(ord))
    }

    pub fn cas(&self, curr: Ptr<T>, new: Ptr<T>, ord: Ordering) -> Result<(), Ptr<T>> {
        let prev = self.inner.compare_and_swap(
            curr.merge(),
            new.merge(),
            ord,
        );

        if prev == curr.merge() {
            Ok(())
        } else {
            Err(curr)
        }
    }
}

impl<T> Ptr<T> {
    pub fn new(inner: *mut T) -> Self {
        Ptr {
            flag: 0,
            inner,
        }
    }

    pub fn null() -> Self {
        Ptr {
            flag: 0,
            inner: ::std::ptr::null_mut(),
        }
    }

    pub unsafe fn into_box(self) -> Option<Box<T>> {
        if self.inner.is_null() {
            None
        } else {
            Some(Box::from_raw(self.inner))
        }
    }

    pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        self.inner.as_ref()
    }

    pub unsafe fn as_mut<'a>(self) -> Option<&'a mut T> {
        self.inner.as_mut()
    }

    /// Increment inner ABA flag.
    pub fn next(mut self) -> Self {
        self.flag += 1;
        self.flag &= FLAG;
        self
    }

    fn restore(value: usize) -> Self {
        Ptr {
            flag: value & FLAG,
            inner: (value ^ FLAG) as *mut T,
        }
    }

    fn merge(self) -> usize {
        self.inner as usize | self.flag
    }
}

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        Ptr {
            flag: self.flag,
            inner: self.inner,
        }
    }
}

impl<T> Copy for Ptr<T> {}

impl<T> PartialEq for Ptr<T> {
    fn eq(&self, other: &Self) -> bool {
        ::std::ptr::eq(self.inner, other.inner) && self.flag == other.flag
    }
}
