
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

#[derive(Debug)]
pub struct AtomicCell<T> {
    ptr: AtomicPtr<T>,
}

impl<T> AtomicCell<T> {
    pub fn new(boxed: Box<T>) -> Self {
        AtomicCell {
            ptr: Box::into_raw(boxed).into()
        }
    }

    pub fn swap(&self, boxed: &mut Box<T>) {
        let box_ptr = self.ptr.swap(&mut **boxed, Ordering::Relaxed);

        unsafe {
            ptr::write(boxed, Box::from_raw(box_ptr));
        }
    }

    pub fn replace(&self, mut boxed: Box<T>) -> Box<T> {
        self.swap(&mut boxed);
        boxed
    }

    pub fn get_mut(&mut self) -> &mut T {
        let ptr = self.ptr.get_mut();

        unsafe {
            &mut **ptr
        }
    }

    pub fn into_box(self) -> Box<T> {
        unsafe {
            Box::from_raw(self.ptr.into_inner())
        }
    }
}

impl<T: Default> Default for AtomicCell<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}
