
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::cell::Cell;
use std::ops::Drop;

use role::{self, Kind};
use buffer::Buffer;
use sequence::Sequence;

use queue::head::Head;

use self::atomic::{AtomicArc, AtomicState::{Empty, Closed, Holds}};

#[derive(Debug)]
pub(crate) struct Chain<S, R, T> where
    S: Sequence,
    R: Sequence,
{
    inner: Arc<Inner<S, R, T>>,
    live: Arc<LiveCount>,
    kind: role::Kind,
    cache_closed: Cell<bool>,
}

#[derive(Debug)]
struct Inner<S: Sequence, R: Sequence, T> {
    buf: Option<Buffer<Arc<Head<S, R>>, T>>,
    next: AtomicArc<Inner<S, R, T>>,
}

#[derive(Debug)]
struct LiveCount {
    sender: AtomicUsize,
    receiver: AtomicUsize,
}

pub(crate) fn pair<S: Sequence, R: Sequence, T>() -> (Chain<S, R, T>, Chain<S, R, T>) {
    let inner = Arc::new(Inner {
        buf: None,
        next: AtomicArc::new(),
    });
    let live = Arc::new(LiveCount {
        sender: 1.into(),
        receiver: 1.into(),
    });

    let sender = Chain {
        inner: inner.clone(),
        live: live.clone(),
        kind: Kind::Send,
        cache_closed: false.into(),
    };
    let receiver = Chain {
        inner,
        live,
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
                Empty | Closed => return,
                Holds(next) => inner = next,
            }
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
        let live = match self.kind {
            Kind::Send => &self.live.sender,
            Kind::Receive => &self.live.receiver,
        };

        live.fetch_add(1, Ordering::Relaxed);

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
        let live = match self.kind {
            Kind::Send => &self.live.sender,
            Kind::Receive => &self.live.receiver,
        };

        if live.fetch_sub(1, Ordering::Release) == 1 {
            self.close();
        }
    }
}

mod atomic {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicPtr, Ordering};
    use std::ptr::{self, NonNull};
    use std::mem;

    #[derive(Debug)]
    pub struct AtomicArc<T> {
        ptr: AtomicPtr<T>,
    }

    #[derive(Debug)]
    pub enum AtomicState<T> {
        Empty,
        Closed,
        Holds(Arc<T>),
    }

    fn get_state<T>(raw: *mut T) -> AtomicState<T> {
        match NonNull::new(raw) {
            None => AtomicState::Empty,
            Some(non_null) => {
                if non_null == NonNull::dangling() {
                    AtomicState::Closed
                } else {
                    let arc = unsafe { Arc::from_raw(non_null.as_ptr()) };
                    mem::forget(arc.clone());
                    AtomicState::Holds(arc)
                }
            }
        }
    }

    impl<T> AtomicArc<T> {
        pub fn new() -> Self {
            AtomicArc {
                ptr: AtomicPtr::new(ptr::null_mut()),
            }
        }

        /// Get current state.
        pub fn fetch(&self) -> AtomicState<T> {
            let inner = self.ptr.load(Ordering::Acquire);
            get_state(inner)
        }

        /// Initialize with given value if empty. Returns previous state.
        pub fn init(&self, value: Arc<T>) -> AtomicState<T> {
            let value = Arc::into_raw(value) as *mut T;
            let prev = self.ptr.compare_and_swap(ptr::null_mut(), value, Ordering::AcqRel);
            let state = get_state(prev);

            match state {
                AtomicState::Empty => {}
                _ => {
                    // value is not consumed so drop it
                    drop(unsafe {
                        Arc::from_raw(value)
                    });
                }
            }

            state
        }

        /// Close if empty. Returns previous state.
        pub fn close(&self) -> AtomicState<T> {
            let dangling = NonNull::dangling().as_ptr();
            let prev = self.ptr.compare_and_swap(ptr::null_mut(), dangling, Ordering::AcqRel);
            get_state(prev)
        }
    }
}
