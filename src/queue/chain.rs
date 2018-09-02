
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::cell::Cell;
use std::ops::Drop;

use role::{self, Kind};
use buffer::Buffer;
use sequence::Sequence;

use queue::head::Head;
use queue::atomic::{AtomicArc, AtomicState::{Empty, Closed, Holds}};

#[derive(Debug)]
pub(crate) struct Chain<S, R, T> where
    S: Sequence,
    R: Sequence,
{
    inner: Arc<Inner<S, R, T>>,
    live: Arc<AtomicUsize>,
    kind: role::Kind,
    cache_closed: Cell<bool>,
}

#[derive(Debug)]
struct Inner<S: Sequence, R: Sequence, T> {
    buf: Option<Buffer<Arc<Head<S, R>>, T>>,
    next: AtomicArc<Inner<S, R, T>>,
}

pub(crate) fn pair<S: Sequence, R: Sequence, T>() -> (Chain<S, R, T>, Chain<S, R, T>) {
    let inner = Arc::new(Inner {
        buf: None,
        next: AtomicArc::new(),
    });

    let sender = Chain {
        inner: inner.clone(),
        live: Arc::new(1.into()),
        kind: Kind::Send,
        cache_closed: false.into(),
    };
    let receiver = Chain {
        inner,
        live: Arc::new(1.into()),
        kind: Kind::Receive,
        cache_closed: false.into(),
    };

    (sender, receiver)
}

impl<S: Sequence, R: Sequence, T> Chain<S, R, T> {
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
                Empty | Closed => break,
                Holds(next) => inner = next,
            }
        }

        if let Some(buf) = &inner.buf {
            buf.head().close();
        }
    }

    pub fn buf(&self) -> Option<&Buffer<Arc<Head<S, R>>, T>> {
        self.inner.buf.as_ref()
    }

    pub fn goto_next(&mut self) -> bool {
        match self.inner.next.fetch() {
            Empty | Closed => false,
            Holds(next) => {
                self.inner = next;
                true
            }
        }
    }

    pub fn goto_last(&mut self) {
        while self.goto_next() {}
    }

    pub fn grow(&mut self) {
        let capacity = self.buf().map_or(1, |buf| buf.capacity() * 2);

        let head = Head::new(S::default(), R::default());
        let buf = Buffer::new(head, capacity);

        let inner = Arc::new(Inner {
            buf: Some(buf),
            next: AtomicArc::new(),
        });

        let prev = self.inner.clone();
        match prev.next.init(inner.clone()) {
            Closed => {}
            Empty => self.inner = inner,
            Holds(next) => self.inner = next,
        }
    }
}

impl<S: Sequence, R: Sequence, T> Clone for Chain<S, R, T> {
    fn clone(&self) -> Self {
        let old_refcount = self.live.fetch_add(1, Ordering::Relaxed);

        if old_refcount > ::std::isize::MAX as usize {
            ::std::process::abort();
        }

        Chain {
            inner: self.inner.clone(),
            kind: self.kind,
            live: self.live.clone(),
            cache_closed: false.into(),
        }
    }
}

impl<S: Sequence, R: Sequence, T> Drop for Chain<S, R, T> {
    fn drop(&mut self) {
        let live_count = self.live.fetch_sub(1, Ordering::Release);
        if live_count == 1 {
            self.close();
            println!("Closed on drop!");
        } else {
            println!("Not closed on drop! live_count: {}", live_count);
        }
    }
}
