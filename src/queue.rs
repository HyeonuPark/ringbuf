
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::{self, Thread};

use counter::Counter;
use sequence::{Sequence, Limit};
use buffer::{Buffer, BufInfo};

#[derive(Debug)]
struct Head<S: Sequence, R: Sequence> {
    sender: Side<S>,
    receiver: Side<R>,
}

#[derive(Debug, Default)]
struct Side<S: Sequence> {
    seq: S,
    count: AtomicUsize,
}

impl<S: Sequence, R: Sequence> BufInfo for Arc<Head<S, R>> {
    fn start(&self) -> Counter {
        self.receiver.seq.count()
    }

    fn end(&self) -> Counter {
        self.sender.seq.count()
    }
}

#[derive(Debug)]
struct Buf<S: Sequence, R: Sequence, T: Send> {
    capacity: usize,
    data: Buffer<Arc<Head<S, R>>, T>,
    sender: Buffer<Arc<Head<S, R>>, Signal>,
    receiver: Buffer<Arc<Head<S, R>>, Signal>,
}

impl<S: Sequence, R: Sequence, T: Send> Clone for Buf<S, R, T> {
    fn clone(&self) -> Self {
        Buf {
            capacity: self.capacity,
            data: self.data.clone(),
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Sender<S: Sequence, R: Sequence, T: Send> {
    head: Arc<Head<S, R>>,
    cache: S::Cache,
    buf: Buf<S, R, T>,

}

#[derive(Debug)]
pub struct Receiver<S: Sequence, R: Sequence, T: Send> {
    head: Arc<Head<S, R>>,
    cache: R::Cache,
    buf: Buf<S, R, T>,
}

#[derive(Debug)]
pub struct SendError<T>(pub T);

#[derive(Debug)]
pub struct ReceiveError;

#[derive(Debug)]
struct SenderLimit<'a, R: Sequence + 'a>(usize, &'a R);

impl<'a, R: Sequence> Limit<'a> for SenderLimit<'a, R> {
    fn count(&self) -> Counter {
        self.1.count() + self.0
    }
}

#[derive(Debug)]
struct ReceiverLimit<'a, S: Sequence + 'a>(&'a S);

impl<'a, S: Sequence> Limit<'a> for ReceiverLimit<'a, S> {
    fn count(&self) -> Counter {
        self.0.count()
    }
}

pub fn create<S: Sequence, R: Sequence, T: Send>(
    capacity: usize
) -> (Sender<S, R, T>, Receiver<S, R, T>) {
    let head: Arc<Head<S, R>> = Arc::new(Head {
        sender: Side::default(),
        receiver: Side::default(),
    });

    let sender = Buffer::new(head.clone(), capacity);
    let receiver = Buffer::new(head.clone(), capacity);

    for buf in &[&sender, &receiver] {
        for i in 0..capacity {
            unsafe {
                buf.write(Counter::new(i), Signal::default());
            }
        }
    }

    let buf = Buf {
        capacity,
        data: Buffer::new(head.clone(), capacity),
        sender,
        receiver,
    };

    let sender = Sender {
        buf: buf.clone(),
        cache: head.sender.seq.cache(SenderLimit(capacity, &head.receiver.seq)),
        head: head.clone(),
    };
    let receiver = Receiver {
        buf,
        cache: head.receiver.seq.cache(ReceiverLimit(&head.sender.seq)),
        head,
    };

    (sender, receiver)
}

#[derive(Default)]
struct Signal(AtomicUsize);

enum Blocked {
    Sync(Thread),
}

const SIGNAL_EMPTY: usize = 0;
const SIGNAL_WAKEN: usize = 1;

impl Signal {
    fn wake(&self) {
        let signal = self.0.swap(SIGNAL_WAKEN, Ordering::AcqRel);

        match signal {
            SIGNAL_EMPTY => {}
            SIGNAL_WAKEN => unreachable!("Signal get double-wake"),
            other => unsafe {
                let signal = other as *const Blocked;

                match *signal {
                    Blocked::Sync(ref thread) => {
                        thread.unpark();
                    }
                }
            }
        }
    }

    fn block(&self, blocked: &Blocked) -> bool {
        let signal = self.0.swap(blocked as *const Blocked as usize, Ordering::AcqRel);

        match signal {
            SIGNAL_EMPTY => true,
            SIGNAL_WAKEN => false,
            _ => unreachable!("Signal get double-block"),
        }
    }

    fn clear(&self) {
        let signal = self.0.swap(SIGNAL_EMPTY, Ordering::AcqRel);
        debug_assert_ne!(signal, SIGNAL_EMPTY, "Signal get double-clear");
    }
}

macro_rules! common_logic {
    (
        $Name:ident
        ($this:ident, $T:ident, $input:ident, $count:ident, $near:ident, $far:ident)
        ($Input:ty, $Output:ty)
        ($limit:expr)
        ($action:expr)
    ) => (
        #[allow(unused_variables)]
        impl<S: Sequence, R: Sequence, $T: Send> $Name<S, R, $T> {
            fn try_advance(
                &mut self, $input: $Input
            ) -> Result<$Output, $Input> {
                let $this = self;
                let limit = $limit;
                let seq = &$this.head.$near.seq;
                let buf = &$this.buf;
                let cache = &mut $this.cache;

                match seq.try_claim(cache, limit) {
                    Some($count) => unsafe {
                        let res = $action;
                        seq.commit(cache, $count);
                        buf.$far.get($count).wake();
                        Ok(res)
                    }
                    None => Err($input),
                }
            }

            fn sync_advance(
                &mut self, $input: $Input
            ) -> $Output {
                let $this = self;
                let limit = $limit;
                let seq = &$this.head.$near.seq;
                let buf = &$this.buf;
                let cache = &mut $this.cache;

                match seq.claim(cache, limit) {
                    Ok($count) => unsafe {
                        let res = $action;
                        seq.commit(cache, $count);
                        buf.$far.get($count).wake();
                        res
                    }
                    Err($count) => unsafe {
                        let blocked = Blocked::Sync(thread::current());
                        let signal = buf.$near.get($count);

                        if signal.block(&blocked) {
                            thread::park();
                        }

                        signal.clear();

                        let res = $action;
                        seq.commit(cache, $count);
                        buf.$far.get($count).wake();
                        res
                    }
                }
            }
        }
    );
}

common_logic! {
    Sender
    (this, T, input, count, sender, receiver)
    (T, ())
    (SenderLimit(this.buf.capacity, &this.head.receiver.seq))
    (this.buf.data.write(count, input))
}

impl<S: Sequence, R: Sequence, T: Send> Sender<S, R, T> {
    pub fn try_send(&mut self, msg: T) -> Result<(), SendError<T>> {
        self.try_advance(msg).map_err(|msg| SendError(msg))
    }

    pub fn sync_send(&mut self, msg: T) {
        self.sync_advance(msg);
    }
}

common_logic! {
    Receiver
    (this, T, input, count, receiver, sender)
    ((), T)
    (ReceiverLimit(&this.head.receiver.seq))
    (this.buf.data.read(count))
}

impl<S: Sequence, R: Sequence, T: Send> Receiver<S, R, T> {
    pub fn try_recv(&mut self) -> Result<T, ReceiveError> {
        self.try_advance(()).map_err(|_| ReceiveError)
    }

    pub fn sync_recv(&mut self) -> T {
        self.sync_advance(())
    }
}
