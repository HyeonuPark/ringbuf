
use std::mem::{self, ManuallyDrop};
use std::cell::UnsafeCell;
use std::ptr;

#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct Slot<T> {
    slot: ManuallyDrop<UnsafeCell<T>>,

    #[cfg(debug_assertions)]
    is_filled: AtomicBool,
}

impl<T> Slot<T> {
    pub fn new() -> Self {
        Slot {
            slot: unsafe { mem::uninitialized() },

            #[cfg(debug_assertions)]
            is_filled: AtomicBool::new(false),
        }
    }

    #[cfg(debug_assertions)]
    fn before_read(&self) {
        assert!(self.is_filled.load(Ordering::Acquire));
    }

    #[cfg(debug_assertions)]
    fn before_write(&self) {
        assert!(!self.is_filled.load(Ordering::Acquire));
    }

    #[cfg(debug_assertions)]
    fn after_read(&self) {
        assert!(self.is_filled.swap(false, Ordering::Release));
    }

    #[cfg(debug_assertions)]
    fn after_write(&self) {
        assert!(!self.is_filled.swap(true, Ordering::Release));
    }

    pub unsafe fn read_from(&self, location: *const T) {
        #[cfg(debug_assertions)]
        self.before_write();

        ptr::copy_nonoverlapping(location, self.slot.get(), 1);

        #[cfg(debug_assertions)]
        self.after_write();
    }

    pub unsafe fn write_to(&self, location: *mut T) {
        #[cfg(debug_assertions)]
        self.before_read();

        ptr::copy_nonoverlapping(self.slot.get(), location, 1);

        #[cfg(debug_assertions)]
        self.after_read();
    }

    pub fn read(&self) -> T {
        #[cfg(debug_assertions)]
        self.before_read();

        let res = unsafe {
            ptr::read(self.slot.get())
        };

        #[cfg(debug_assertions)]
        self.after_read();

        res
    }

    pub fn write(&self, value: T) {
        #[cfg(debug_assertions)]
        self.before_write();

        unsafe {
            ptr::write(self.slot.get(), value);
        }

        #[cfg(debug_assertions)]
        self.after_write();
    }

    pub fn drop_in_place(&self) {
        #[cfg(debug_assertions)]
        self.before_read();

        unsafe {
            ptr::drop_in_place(self.slot.get());
        }

        #[cfg(debug_assertions)]
        self.after_read();
    }
}
