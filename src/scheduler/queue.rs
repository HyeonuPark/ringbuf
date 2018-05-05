
use std::sync::atomic::{AtomicPtr, Ordering as O};
use std::ptr;
use std::mem::{self, ManuallyDrop};
use std::ops::Drop;

use role::Kind;
use scheduler::{Notify, Slot};

#[derive(Debug)]
pub struct Queue {
    head: Atomic,
    tail: ManuallyDrop<Atomic>,
}

#[derive(Debug)]
pub struct NodeBox(Box<Node>);

#[derive(Debug)]
struct Node {
    info: Option<Info>,
    next: Atomic,
}

#[derive(Debug)]
struct Info {
    role: Kind,
    slot: Slot<NodeBox>,
    notify: Notify,
}

#[derive(Debug)]
#[repr(align(64))]
struct Atomic {
    ptr: AtomicPtr<Node>,
}

impl Atomic {
    fn new(value: *mut Node) -> Self {
        Atomic {
            ptr: AtomicPtr::new(value),
        }
    }

    fn null() -> Self {
        Atomic::new(ptr::null_mut())
    }

    fn load(&self) -> *mut Node {
        self.ptr.load(O::Acquire)
    }

    fn cas(&self, cond: *mut Node, value: *mut Node) -> Result<(), *mut Node> {
        let prev = self.ptr.compare_and_swap(cond, value, O::Release);

        if ptr::eq(prev, cond) {
            Ok(())
        } else {
            Err(prev)
        }
    }
}

impl Drop for Atomic {
    fn drop(&mut self) {
        let node = self.load();

        if !node.is_null() {
            unsafe {
                ptr::drop_in_place(node);
            }
        }
    }
}

impl Node {
    fn mismatch_with(&self, role: Kind) -> bool {
        match self.info {
            None => true,
            Some(ref info) => info.role != role,
        }
    }
}

impl NodeBox {
    pub fn new() -> Self {
        NodeBox(Box::new(Node {
            info: None,
            next: Atomic::null(),
        }))
    }

    fn ptr(&self) -> *mut Node {
        &*self.0 as *const Node as *mut Node
    }

    fn leak(self) {
        mem::forget(self);
    }

    unsafe fn recover(node: *mut Node) -> Self {
        NodeBox(Box::from_raw(node))
    }
}

impl Queue {
    pub fn new() -> Self {
        let sentinel = NodeBox::new();
        let ptr = sentinel.ptr();
        sentinel.leak();

        Queue {
            head: Atomic::new(ptr),
            tail: ManuallyDrop::new(Atomic::new(ptr)),
        }
    }

    pub fn push(
        &self, role: Kind, notify: Notify, mut node_box: NodeBox, slot: Slot<NodeBox>
    ) -> Result<(), NodeBox> {
        node_box.0.info = Some(Info {
            role,
            slot,
            notify,
        });
        let node = node_box.ptr();

        loop {
            let head = self.head.load();
            let tail = self.tail.load();
            let tail_ref = unsafe { &*tail };

            if !ptr::eq(head, tail) && tail_ref.mismatch_with(role) {
                return Err(node_box);
            }

            let next = tail_ref.next.load();
            match unsafe { next.as_ref() } {
                Some(next_ref) => {
                    if next_ref.mismatch_with(role) {
                        return Err(node_box);
                    }

                    let _ = self.tail.cas(tail, next);
                }
                None => {
                    if tail_ref.next.cas(ptr::null_mut(), node).is_ok() {
                        let _ = self.tail.cas(tail, node);
                        node_box.leak();
                        return Ok(());
                    }
                }
            }
        }
    }

    pub fn pop(&self, role: Kind) -> Option<Notify> {
        loop {
            let head = self.head.load();
            let next = unsafe { &*head }.next.load();

            let next_ref = unsafe { next.as_mut() }?;

            if next_ref.mismatch_with(role) {
                return None;
            }

            let info = unsafe { ptr::read(&next_ref.info) };

            if self.head.cas(head, next).is_ok() {
                let info = info.unwrap();
                unsafe {
                    let node_box = NodeBox::recover(head);
                    info.slot.write(node_box);
                }
                return Some(info.notify);
            } else {
                mem::forget(info);
            }
        }
    }
}
