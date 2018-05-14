
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicBool, AtomicPtr, Ordering};
use std::{mem, ptr};
use std::ops::Drop;

use counter::Counter;
use sequence::{Sequence, Limit};
use buffer::BufInfo;
use role;

use super::half::HeadHalf;

#[derive(Debug)]
pub struct Head<S: Sequence, R: Sequence> {
    sender: S,
    sender_count: AtomicUsize,
    receiver: R,
    receiver_count: AtomicUsize,
    is_closed: AtomicBool,
    next: AtomicPtr<Head<S, R>>,
}

#[derive(Debug)]
pub struct SenderHead<S: Sequence, R: Sequence, T> {
    head: Arc<Head<S, R>>,
    capacity: usize,
    role: role::Sender<T>,
}

#[derive(Debug)]
pub struct ReceiverHead<S: Sequence, R: Sequence, T> {
    head: Arc<Head<S, R>>,
    role: role::Receiver<T>,
}

impl<S: Sequence, R: Sequence> Head<S, R> {
    pub fn new(sender: S, receiver: R) -> Self {
        Head {
            sender,
            sender_count: AtomicUsize::new(1),
            receiver,
            receiver_count: AtomicUsize::new(1),
            is_closed: AtomicBool::new(false),
            next: AtomicPtr::new(ptr::null_mut()),
        }
    }

    pub fn get_next(&self) -> Option<Arc<Self>> {
        let next = self.next.load(Ordering::Acquire);

        if next.is_null() {
            None
        } else {
            Some(unsafe { clone_arc(next) })
        }
    }

    pub fn set_next(&self, next: Arc<Self>) -> Result<(), Arc<Self>> {
        let next = Arc::into_raw(next) as *mut Self;
        let prev = self.next.compare_and_swap(ptr::null_mut(), next, Ordering::Acquire);

        if prev.is_null() {
            Ok(())
        } else {
            Err(unsafe { clone_arc(prev) })
        }
    }
}

impl<S: Sequence, R: Sequence> Drop for Head<S, R> {
    fn drop(&mut self) {
        let next = self.next.load(Ordering::Acquire);

        if !next.is_null() {
            unsafe {
                Arc::from_raw(next);
            }
        }
    }
}

impl<S: Sequence, R: Sequence> BufInfo for Arc<Head<S, R>> {
    fn range(&self) -> (Counter, Counter) {
        (self.receiver.count(), self.sender.count())
    }
}

impl<S: Sequence, R: Sequence, T> SenderHead<S, R, T> {
    pub fn new(head: Arc<Head<S, R>>, capacity: usize) -> Self {
        SenderHead {
            head,
            capacity,
            role: Default::default(),
        }
    }
}

impl<S: Sequence, R: Sequence, T> HeadHalf for SenderHead<S, R, T> {
    type Seq = S;
    type Role = role::Sender<T>;

    fn seq(&self) -> &S {
        &self.head.sender
    }

    fn amount(&self) -> &AtomicUsize {
        &self.head.sender_count
    }

    fn is_closed(&self) -> &AtomicBool {
        &self.head.is_closed
    }
}

impl<S: Sequence, R: Sequence, T> Limit for SenderHead<S, R, T> {
    fn count(&self) -> Counter {
        self.head.receiver.count() + self.capacity
    }
}

impl<S: Sequence, R: Sequence, T> Clone for SenderHead<S, R, T> {
    fn clone(&self) -> Self {
        SenderHead {
            head: self.head.clone(),
            role: self.role,
            capacity: self.capacity,
        }
    }
}

impl<S: Sequence, R: Sequence, T> ReceiverHead<S, R, T> {
    pub fn new(head: Arc<Head<S, R>>) -> Self {
        ReceiverHead {
            head,
            role: Default::default(),
        }
    }
}

impl<S: Sequence, R: Sequence, T> HeadHalf for ReceiverHead<S, R, T> {
    type Seq = R;
    type Role = role::Receiver<T>;

    fn seq(&self) -> &R {
        &self.head.receiver
    }

    fn amount(&self) -> &AtomicUsize {
        &self.head.receiver_count
    }

    fn is_closed(&self) -> &AtomicBool {
        &self.head.is_closed
    }
}

impl<S: Sequence, R: Sequence, T> Limit for ReceiverHead<S, R, T> {
    fn count(&self) -> Counter {
        self.head.sender.count()
    }
}

impl<S: Sequence, R: Sequence, T> Clone for ReceiverHead<S, R, T> {
    fn clone(&self) -> Self {
        ReceiverHead {
            head: self.head.clone(),
            role: self.role,
        }
    }
}

unsafe fn clone_arc<T>(arc_ptr: *mut T) -> Arc<T> {
    let arc = Arc::from_raw(arc_ptr);
    let res = arc.clone();
    mem::forget(arc);
    res
}
