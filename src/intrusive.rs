
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;
use std::ops::{Deref, DerefMut};

use counter::{Counter, AtomicCounter};

/// An intrusive treiber stack.
///
/// Users need to explicitly provide `Arc<Node<T>>` to this stack.
#[derive(Debug)]
pub struct Stack<T> {
    head: AtomicPtr<Node<T>>,
}

#[derive(Debug)]
pub struct Node<T> {
    value: T,
    stamp: AtomicCounter,
    next: AtomicPtr<Node<T>>,
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Stack {
            head: AtomicPtr::new(ptr::null_mut()),
        }
    }

    pub fn push(&self, node: Arc<Node<T>>) {
        node.stamp.incr(1); // To avoid ABA problem.

        let next = Arc::into_raw(node.clone()) as *mut Node<T>;
        let mut prev = self.head.load(Ordering::Relaxed);
        let mut prev_stamp = fetch_stamp(prev);

        loop {
            node.next.store(prev, Ordering::Relaxed);

            let swap = self.head.compare_and_swap(prev, next, Ordering::Relaxed);
            let swap_stamp = fetch_stamp(swap);

            if ptr::eq(prev, swap) && prev_stamp == swap_stamp {
                return
            } else {
                prev = swap;
                prev_stamp = swap_stamp;
            }
        }
    }

    pub fn pop(&self) -> Option<Arc<Node<T>>> {
        loop {
            let prev = self.head.load(Ordering::Relaxed);

            match unsafe { prev.as_ref() } {
                None => return None,
                Some(prev_node) => {
                    let prev_stamp = fetch_stamp(prev);
                    let next = prev_node.next.load(Ordering::Relaxed);
                    let swap = self.head.compare_and_swap(prev, next, Ordering::Acquire);
                    let swap_stamp = fetch_stamp(swap);

                    if ptr::eq(prev, swap) && prev_stamp == swap_stamp {
                        let prev_node = unsafe { Arc::from_raw(prev) };
                        return Some(prev_node);
                    }
                }
            }
        }
    }
}

impl<T> Node<T> {
    pub fn new(value: T) -> Arc<Self> {
        Arc::new(Node {
            value,
            stamp: AtomicCounter::new(),
            next: AtomicPtr::new(ptr::null_mut()),
        })
    }
}

impl<T> Deref for Node<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for Node<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

fn fetch_stamp<T>(ptr: *mut Node<T>) -> Option<Counter> {
    unsafe {
        ptr.as_ref().map(|node| node.stamp.fetch())
    }
}
