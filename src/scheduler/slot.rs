
use std::sync::Arc;
use std::mem::{self, ManuallyDrop};
use std::cell::UnsafeCell;
use std::ptr;

#[derive(Debug)]
pub struct Slot<T> {
    slot: Arc<ManuallyDrop<UnsafeCell<T>>>,
}

impl<T> Slot<T> {
    pub fn new() -> Self {
        Slot {
            slot: Arc::new(unsafe { mem::zeroed() }),
        }
    }

    pub unsafe fn read(&self) -> T {
        ptr::read(self.slot.get())
    }

    pub unsafe fn write(&self, value: T) {
        ptr::write(self.slot.get(), value);
    }
}

impl<T> Clone for Slot<T> {
    fn clone(&self) -> Self {
        Slot {
            slot: self.slot.clone(),
        }
    }
}
