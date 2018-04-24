//! Thread safe generalized growable queue implementation
//! using [bounded queue](../bounded/index.html) underneath.
#![allow(missing_docs)]
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::{ptr, mem};

use sequence::{Sequence, Shared};
use ringbuf::RingBuf;
use super::bounded::{Head, Sender as BoundSender, Receiver as BoundReceiver};
use super::bounded::{SendError, queue_buf};

#[derive(Debug)]
struct Inner<S: Sequence, R: Sequence, T: Send> {
    buf: RingBuf<Head<S, R>, T>,
    upgrade: AtomicPtr<Inner<S, R, T>>,
}

/// The transmission end of the growable queue.
#[derive(Debug)]
pub struct Sender<S: Sequence, R: Sequence, T: Send> {
    inner: Arc<Inner<S, R, T>>,
    bound: Option<BoundSender<S, R, T>>,
}

/// The receiving end of the growable queue.
#[derive(Debug)]
pub struct Receiver<S: Sequence, R: Sequence, T: Send> {
    inner: Arc<Inner<S, R, T>>,
    bound: Option<BoundReceiver<S, R, T>>,
}

unsafe impl<S: Sequence, R: Sequence, T: Send> Send for Sender<S, R, T> {}
unsafe impl<S: Sequence, R: Sequence, T: Send> Send for Receiver<S, R, T> {}

/// Creates a growable queue.
pub fn queue<S: Sequence, R: Sequence, T: Send>() -> (Sender<S, R, T>, Receiver<S, R, T>) {
    let inner = Arc::new(Inner {
        buf: queue_buf(1),
        upgrade: AtomicPtr::new(ptr::null_mut()),
    });

    let sender = Sender {
        inner: inner.clone(),
        bound: None,
    };
    let receiver = Receiver {
        inner,
        bound: None,
    };

    (sender, receiver)
}

impl<S: Sequence, R: Sequence, T: Send> Sender<S, R, T> {
    pub fn send(&mut self, msg: T) {
        if self.bound.is_none() {
            upgrade(&mut self.inner);
            let buf = self.inner.buf.clone();

            self.bound = Some(BoundSender::new(buf));
        }

        defmac!(sender => self.bound.as_mut().unwrap());
        let mut message = msg;

        loop {
            match sender!().try_send(message) {
                Ok(()) => return,
                Err(SendError { msg, .. }) => {
                    message = msg;

                    // Check if buffer already upgraded
                    if upgrade(&mut self.inner) {
                        continue;
                    }

                    self.inner.buf.close();

                    // Construct new buffer
                    let capacity = sender!().capacity() * 2;
                    let buf = queue_buf(capacity);
                    let inner = Arc::new(Inner {
                        buf,
                        upgrade: AtomicPtr::new(ptr::null_mut()),
                    });
                    let inner_ptr = Arc::into_raw(inner.clone());

                    let swap = self.inner.upgrade.compare_and_swap(
                        ptr::null_mut(), inner_ptr as _, Ordering::AcqRel);

                    if swap == ptr::null_mut() {
                        // Replace succeeded.
                        self.bound = Some(BoundSender::new(inner.buf.clone()));
                        self.inner = inner;
                    } else {
                        // Someone else already replaced buffer.
                        unsafe { Arc::from_raw(inner_ptr) };
                        upgrade(&mut self.inner);
                    }
                }
            }
        }
    }
}

impl<R: Sequence, T: Send> Clone for Sender<Shared, R, T> {
    fn clone(&self) -> Self {
        Sender {
            inner: self.inner.clone(),
            bound: None,
        }
    }
}

impl<S: Sequence, R: Sequence, T: Send> Receiver<S, R, T> {
    pub fn try_recv(&mut self) -> Option<T> {
        if self.bound.is_none() {
            upgrade(&mut self.inner);
            let buf = self.inner.buf.clone();

            self.bound = Some(BoundReceiver::new(buf));
        }

        defmac!(receiver => self.bound.as_mut().unwrap());

        loop {
            match receiver!().try_recv() {
                Some(msg) => return Some(msg),
                None => {
                    if !upgrade(&mut self.inner) {
                        return None;
                    }
                }
            }
        }
    }
}

impl<S: Sequence, T: Send> Clone for Receiver<S, Shared, T> {
    fn clone(&self) -> Self {
        Receiver {
            inner: self.inner.clone(),
            bound: None,
        }
    }
}

/// returns true if upgrade succeeded.
fn upgrade<S: Sequence, R: Sequence, T: Send>(inner: &mut Arc<Inner<S, R, T>>) -> bool {
    let mut succeeded = false;

    loop {
        let upgrade = inner.upgrade.load(Ordering::Acquire);

        if upgrade.is_null() {
            return succeeded;
        }

        succeeded = true;

        *inner = unsafe {
            let remote = Arc::from_raw(upgrade);
            mem::forget(remote.clone());
            remote
        };
    }
}
