
use std::sync::atomic::Ordering as O;
use std::ptr;

use super::{Role, Handle};
use super::atomic::{Atomic, Ptr};

/// Queue that holds blocked senders or receivers.
///
/// The core invariance of this queue is, it MUST NOT contains
/// both senders and receivers at the same time.
///
/// Also, this is an implementation of Michael-scott queue,
/// so their invariances MUST be guaranteed.
#[derive(Debug)]
pub struct Queue<T> {
    head: Atomic<Node<T>>,
    tail: Atomic<Node<T>>,
}

#[derive(Debug)]
pub struct Node<T> {
    role: Role,
    handle: Option<Box<Handle<T>>>,
    next: Atomic<Node<T>>,
}

impl<T> Node<T> {
    /// Allocate a new node
    pub fn new() -> Box<Self> {
        Box::new(Node {
            role: Role::Sender, // whatever
            handle: None,
            next: Atomic::null(),
        })
    }

    pub fn init(&mut self, role: Role, handle: Box<Handle<T>>) {
        self.role = role;
        self.handle = Some(handle);
    }

    pub fn take_handle(&mut self) -> Option<Box<Handle<T>>> {
        self.handle.take()
    }
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        let sentinel = Ptr::new(Box::into_raw(Box::new(Node {
            role: Role::Sender, // whatever
            handle: None,
            next: Atomic::null(),
        })));

        Queue {
            head: Atomic::new(sentinel),
            tail: Atomic::new(sentinel),
        }
    }

    /// Push a node to this queue.
    ///
    /// Returns error with given node if currently the queue holds non-matching roles.
    pub fn push(&self, node: Box<Node<T>>) -> Result<(), Box<Node<T>>> {
        let role = node.role;
        let node = Ptr::new(Box::into_raw(node)).next(); // to avoid ABA problem

        // retry on race condition
        loop {
            let head = self.head.load(O::Acquire);
            let tail = self.tail.load(O::Acquire);
            let tail_ref = unsafe { tail.as_ref() }.unwrap();

            // if this queue is not empty AND has nodes with different role
            // then push operation should be fail to maintain the core invariance.
            if head != tail && tail_ref.role != role {
                return Err(unsafe { node.into_box() }.unwrap());
            }

            // is `tail` the actual tail?
            let next = tail_ref.next.load(O::Acquire);
            match unsafe { next.as_ref() } {
                Some(next_ref) => {
                    // now we now this queue is not empty,
                    // so just checking role is enough.
                    if next_ref.role != role {
                        return Err(unsafe { node.into_box() }.unwrap());
                    }

                    // anyway, the tail pointer is outdated
                    // try to "help" by moving tail pointer forward
                    let _ = self.tail.cas(tail, next, O::Release);
                }
                None => {
                    // attempt to link `node` to `tail`
                    if tail_ref.next.cas(Ptr::null(), node, O::Release).is_ok() {
                        // try to move the tail pointer forward
                        let _ = self.tail.cas(tail, node, O::Release);
                        return Ok(());
                    }
                }
            }
        }
    }

    /// Pop a node with given role, if exist.
    pub fn pop(&self, role: Role) -> Option<Box<Node<T>>> {
        loop {
            let head = self.head.load(O::Acquire);
            let next = unsafe { head.as_ref() }.unwrap().next.load(O::Acquire);

            match unsafe { next.as_mut() } {
                None => return None,
                Some(next_ref) => {
                    if next_ref.role != role {
                        return None;
                    }

                    if self.head.cas(head, next, O::Release).is_ok() {
                        unsafe {
                            let mut res = head.into_box().unwrap();
                            ptr::copy_nonoverlapping(
                                &mut next_ref.handle, &mut res.handle, 1);
                            return Some(res);
                        }
                    }
                }
            }
        }
    }
}
