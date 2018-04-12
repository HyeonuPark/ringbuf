
use std::mem::{size_of, align_of, forget};
use std::marker::{PhantomData};
use std::ptr::{self, NonNull};
use std::ops::Drop;

#[derive(Debug, Clone)]
pub struct RingBuf<H, T> {
    mask: usize,
    head_ptr: NonNull<H>,
    body_ptr: NonNull<T>,
}

impl<H, T> RingBuf<H, T> {
    pub fn new(head: H, capacity: usize) -> Self {
        let size_head = size_of::<H>();
        let size_elem = size_of::<T>();
        let align_head = align_of::<H>();
        let align_elem = align_of::<T>();

        assert!(capacity.is_power_of_two(), "capacity MUST be a power of 2");

        if size_elem == 0 {
            let head = Box::new(head);
            let head_ptr = Box::into_raw(head);

            RingBuf {
                mask: capacity - 1,
                head_ptr: unsafe { NonNull::new_unchecked(head_ptr) },
                body_ptr: NonNull::dangling(),
            }
        } else if size_head == 0 {
            let mut vec: Vec<T> = Vec::with_capacity(capacity);
            let vec_ptr = vec.as_mut_ptr();
            forget(vec);

            RingBuf {
                mask: capacity - 1,
                head_ptr: NonNull::dangling(),
                body_ptr: unsafe { NonNull::new_unchecked(vec_ptr) },
            }
        } else {
            let max_padding = if align_head <= align_elem {
                0
            } else {
                align_head - align_elem
            };

            let head_slot_size = (max_padding + size_head - 1) / size_elem + 1;
            let vec_capacity = head_slot_size + capacity;
            let mut vec: Vec<T> = Vec::with_capacity(vec_capacity);
            let vec_ptr = vec.as_mut_ptr();
            forget(vec);

            unsafe {
                let head_ptr = if max_padding == 0 {
                    vec_ptr as *mut H
                } else {
                    let vec_addr = vec_ptr as usize;
                    let head_addr = (vec_addr - 1 + align_head) ^ (align_head - 1);
                    head_addr as *mut H
                };

                let body_ptr = vec_ptr.offset(head_slot_size as isize);

                RingBuf {
                    mask: capacity - 1,
                    head_ptr: NonNull::new_unchecked(head_ptr),
                    body_ptr: NonNull::new_unchecked(body_ptr),
                }
            }
        }
    }

    pub fn head(&self) -> &H {
        unsafe {
            self.head_ptr.as_ref()
        }
    }

    unsafe fn get_ptr(&self, index: usize) -> *mut T {
        let index = (index & self.mask) as isize;
        self.body_ptr.as_ptr().offset(index as isize)
    }

    pub unsafe fn get(&self, index: usize) -> &T {
        &*self.get_ptr(index)
    }

    pub unsafe fn set(&self, index: usize, value: T) {
        ptr::write(self.get_ptr(index), value)
    }

    pub unsafe fn read(&self, index: usize) -> T {
        ptr::read(self.get_ptr(index))
    }
}

unsafe impl<H: Sync, T: Sync> Sync for RingBuf<H, T> {}

impl<H, T> Drop for RingBuf<H, T> {
    fn drop(&mut self) {
        let size_head = size_of::<H>();
        let size_elem = size_of::<T>();
        let align_head = align_of::<H>();
        let align_elem = align_of::<T>();

        unsafe {
            if size_elem == 0 {
                let _ = Box::from_raw(self.head_ptr.as_ptr());
            } else if size_head == 0 {
                let _ = Vec::from_raw_parts(self.body_ptr.as_ptr(), 0, self.mask + 1);
            } else {
                let max_padding = if align_head <= align_elem {
                    0
                } else {
                    align_head - align_elem
                };

                let head_slot_size = (max_padding + size_head - 1) / size_elem + 1;

                let vec_ptr = self.body_ptr.as_ptr().offset(-(head_slot_size as isize));
                let vec_capacity = self.mask + 1 + head_slot_size;
                let _ = Vec::from_raw_parts(vec_ptr, 0, vec_capacity);
            }
        }
    }
}
