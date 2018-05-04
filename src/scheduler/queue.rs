
use std::sync::atomic::{AtomicPtr, Ordering as O};
use std::ptr;

use role::RoleKind;
use scheduler::Handle;

#[derive(Debug)]
pub struct Queue<T> {
    head: Atomic<T>,
    tail: Atomic<T>,
}

#[derive(Debug)]
pub struct Node<T> {
    role: RoleKind,
    handle: Option<Box<Handle<T>>>,
    next: Atomic<T>,
}

#[derive(Debug)]
#[repr(align(64))]
struct Atomic<T> {
    ptr: AtomicPtr<Node<T>>,
}

impl<T> Atomic<T> {
    fn new(value: *mut Node<T>) -> Self {
        Atomic {
            ptr: AtomicPtr::new(value),
        }
    }

    fn null() -> Self {
        Atomic::new(ptr::null_mut())
    }

    fn load(&self) -> *mut Node<T> {
        self.ptr.load(O::Acquire)
    }

    fn cas(&self, cond: *mut Node<T>, value: *mut Node<T>) -> Result<(), *mut Node<T>> {
        let prev = self.ptr.compare_and_swap(cond, value, O::Release);

        if ptr::eq(prev, cond) {
            Ok(())
        } else {
            Err(prev)
        }
    }
}

impl<T> Node<T> {
    pub fn new() -> Box<Self> {
        Box::new(Node {
            role: RoleKind::Sender,
            handle: None,
            next: Atomic::null(),
        })
    }

    pub fn init(&mut self, role: RoleKind, handle: Box<Handle<T>>) {
        self.role = role;
        self.handle = Some(handle);
    }

    pub fn take_handle(&mut self) -> Option<Box<Handle<T>>> {
        self.handle.take()
    }
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        let sentinel = Box::into_raw(Node::new());

        Queue {
            head: Atomic::new(sentinel),
            tail: Atomic::new(sentinel),
        }
    }

    pub fn push(&self, node: Box<Node<T>>) -> Result<(), Box<Node<T>>> {
        let role = node.role;
        let node = Box::into_raw(node);
        let recover = || unsafe { Box::from_raw(node) };

        loop {
            let head = self.head.load();
            let tail = self.tail.load();
            let tail_ref = unsafe { tail.as_ref() }.unwrap();

            if !ptr::eq(head, tail) && tail_ref.role != role {
                return Err(recover());
            }

            let next = tail_ref.next.load();
            match unsafe { next.as_ref() } {
                Some(next_ref) => {
                    if next_ref.role != role {
                        return Err(recover());
                    }

                    let _ = self.tail.cas(tail, next);
                }
                None => {
                    if tail_ref.next.cas(ptr::null_mut(), node).is_ok() {
                        let _ = self.tail.cas(tail, node);
                        return Ok(());
                    }
                }
            }
        }
    }

    pub fn pop(&self, role: RoleKind) -> Option<Box<Node<T>>> {
        loop {
            let head = self.head.load();
            let next = unsafe { head.as_ref() }.unwrap().next.load();

            let next_ref = unsafe { next.as_mut() }?;

            if next_ref.role != role {
                return None;
            }

            if self.head.cas(head, next).is_ok() {
                unsafe {
                    let mut res = Box::from_raw(head);
                    ptr::copy_nonoverlapping(&mut next_ref.handle, &mut res.handle, 1);
                    return Some(res);
                }
            }
        }
    }
}
