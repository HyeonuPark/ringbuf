
use std::sync::Arc;
use std::cell::Cell;

use sequence::Sequence;
use buffer::Buffer;

use super::head::Head;
use super::state::{Atomic, Next::*};

/// Chain of buffers to provide unbounded queue.
#[derive(Debug)]
pub struct Chain<S: Sequence, R: Sequence, T: Send> {
    inner: Arc<Inner<S, R, T>>,
    cache_closed: Cell<bool>,
}

#[derive(Debug)]
struct Inner<S: Sequence, R: Sequence, T: Send> {
    buf: Option<Buffer<Arc<Head<S, R>>, T>>,
    next: Atomic<Inner<S, R, T>>,
}

impl<S: Sequence, R: Sequence, T: Send> Chain<S, R, T> {
    pub fn new() -> Self {
        let inner = Arc::new(Inner {
            buf: None,
            next: Atomic::new(),
        });

        Chain {
            inner,
            cache_closed: false.into(),
        }
    }

    pub fn is_closed(&self) -> bool {
        if self.cache_closed.get() {
            return true;
        }

        let mut inner = self.inner.clone();

        loop {
            match inner.next.fetch() {
                Empty => return false,
                Holds(next) => inner = next,
                Closed => {
                    self.cache_closed.set(true);
                    return true;
                }
            }
        }
    }

    pub fn close(&self) {
        if self.cache_closed.get() {
            return;
        }

        self.cache_closed.set(true);
        let mut inner = self.inner.clone();

        loop {
            match inner.next.close() {
                Empty | Closed => return,
                Holds(next) => inner = next,
            }
        }
    }

    pub fn buf(&self) -> Option<&Buffer<Arc<Head<S, R>>, T>> {
        self.inner.buf.as_ref()
    }

    pub fn move_next(&mut self) -> bool {
        match self.inner.next.fetch() {
            Empty | Closed => false,
            Holds(next) => {
                self.inner = next;
                true
            }
        }
    }

    pub fn move_last(&mut self) {
        while self.move_next() {}
    }

    pub fn grow(&mut self) {
        let capacity = self.buf().map_or(1, |buf| buf.capacity() * 2);

        let (sender, receiver) = <(S, R)>::default();
        let head = Arc::new(Head::new(sender, receiver));

        let buf = Buffer::new(head, capacity);

        let inner = Arc::new(Inner {
            buf: Some(buf),
            next: Atomic::new(),
        });

        let prev = Arc::clone(&self.inner);
        match prev.next.init(inner.clone()) {
            Closed => return,
            Empty => self.inner = inner,
            Holds(next) => self.inner = next,
        }
    }
}

impl<S: Sequence, R: Sequence, T: Send> Clone for Chain<S, R, T> {
    fn clone(&self) -> Self {
        Chain {
            inner: self.inner.clone(),
            cache_closed: self.cache_closed.clone(),
        }
    }
}
