use std::thread::{self, Thread};
use std::sync::{Mutex, mpsc, Arc};
use std::sync::atomic::{AtomicIsize, Ordering as O};
use std::ops::Drop;

use std::time::{Instant, Duration};

use sequence::Sequence;
use queue::{Sender, Receiver};
use extension::Extension;

#[derive(Debug)]
pub struct Blocking;

#[derive(Debug)]
struct Shared {
    sender_rx: Mutex<mpsc::Receiver<Thread>>,
    receiver_rx: Mutex<mpsc::Receiver<Thread>>,
    // 1: senders are blocked
    // 0: neither end is blocked
    // -1: receivers are blocked
    tristate: AtomicIsize,
}

const BLOCK_SENDERS: isize = 1;
const BLOCK_NONE: isize = 0;
const BLOCK_RECEIVERS: isize = -1;

#[derive(Debug, Clone)]
pub struct SenderLocal {
    shared: Arc<Shared>,
    tx: mpsc::Sender<Thread>,
}

#[derive(Debug, Clone)]
pub struct ReceiverLocal {
    shared: Arc<Shared>,
    tx: mpsc::Sender<Thread>,
}

impl Shared {
    fn register_sender(&self, local: &SenderLocal) {
        local.tx.send(thread::current()).unwrap();

        let prev = Instant::now();
        thread::park_timeout(Duration::from_secs(5));

        if Instant::now() > prev + Duration::from_secs(3) {
            panic!("Sender locked!");
        }
    }

    fn register_receiver(&self, local: &ReceiverLocal) {
        local.tx.send(thread::current()).unwrap();

        let prev = Instant::now();
        thread::park_timeout(Duration::from_secs(5));

        if Instant::now() > prev + Duration::from_secs(3) {
            panic!("Receiver locked!");
        }
    }

    fn notify_senders(&self) {
        use std::sync::TryLockError as TE;

        match self.sender_rx.try_lock() {
            Ok(handles) => {
                let prev = self.tristate.compare_and_swap(BLOCK_SENDERS, BLOCK_NONE, O::AcqRel);

                if prev == BLOCK_SENDERS {
                    match handles.recv() {
                        Ok(handle) => handle.unpark(),
                        Err(_) => return, // Handle channel closed
                    }
                }

                for handle in handles.try_iter() {
                    handle.unpark();
                }
            }
            Err(TE::Poisoned(err)) => {
                panic!("Found poisoned mutex while draining senders: {}", err)
            }
            Err(TE::WouldBlock) => {}
        }
    }

    fn notify_receivers(&self) {
        use std::sync::TryLockError as TE;

        match self.receiver_rx.try_lock() {
            Ok(handles) => {
                let prev = self.tristate.compare_and_swap(BLOCK_RECEIVERS, BLOCK_NONE, O::AcqRel);

                if prev == BLOCK_RECEIVERS {
                    match handles.recv() {
                        Ok(handle) => handle.unpark(),
                        Err(_) => return, // Handle channel closed
                    }
                }

                for handle in handles.try_iter() {
                    handle.unpark();
                }
            }
            Err(TE::Poisoned(err)) => {
                panic!("Found poisoned mutex while draining receivers: {}", err)
            }
            Err(TE::WouldBlock) => {}
        }
    }
}

impl Extension for Blocking {
    type Sender = SenderLocal;
    type Receiver = ReceiverLocal;

    fn create_triple() -> (Self, SenderLocal, ReceiverLocal) {
        let (sender_tx, sender_rx) = mpsc::channel();
        let (receiver_tx, receiver_rx) = mpsc::channel();
        let shared = Arc::new(Shared {
            sender_rx: sender_rx.into(),
            receiver_rx: receiver_rx.into(),
            tristate: 0.into(),
        });

        (
            Blocking,
            SenderLocal {
                shared: shared.clone(),
                tx: sender_tx,
            },
            ReceiverLocal {
                shared,
                tx: receiver_tx,
            },
        )
    }
}

impl Drop for SenderLocal {
    fn drop(&mut self) {
        self.shared.notify_receivers();
    }
}

impl Drop for ReceiverLocal {
    fn drop(&mut self) {
        self.shared.notify_senders();
    }
}

impl<S: Sequence, R: Sequence, T: Send> Sender<S, R, Blocking, T> {
    pub fn send(&mut self, msg: T) -> Result<(), T> {
        use queue::SendErrorKind as K;

        let mut msg = msg;

        loop {
            match self.try_send(msg) {
                Ok(()) => {
                    self.ext().shared.notify_receivers();
                    return Ok(());
                }
                Err(err) => match err.kind {
                    K::ReceiverAllClosed => {
                        return Err(err.payload);
                    }
                    K::BufferFull => {
                        msg = err.payload;

                        let state = &self.ext().shared.tristate;
                        match state.compare_and_swap(BLOCK_NONE, BLOCK_SENDERS, O::AcqRel) {
                            BLOCK_NONE | BLOCK_SENDERS => {
                                self.ext().shared.register_sender(self.ext());
                            }
                            BLOCK_RECEIVERS => {
                                self.ext().shared.notify_receivers();
                                // println!("skip blocking sender");
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
        }
    }
}

impl<S: Sequence, R: Sequence, T: Send> Receiver<S, R, Blocking, T> {
    pub fn recv(&mut self) -> Option<T> {
        loop {
            match self.try_recv() {
                Ok(msg) => {
                    self.ext().shared.notify_senders();
                    return msg;
                }
                Err(_) => {
                    let state = &self.ext().shared.tristate;
                    match state.compare_and_swap(BLOCK_NONE, BLOCK_RECEIVERS, O::AcqRel) {
                        BLOCK_NONE | BLOCK_RECEIVERS => {
                            self.ext().shared.register_receiver(self.ext());
                        }
                        BLOCK_SENDERS => {
                            self.ext().shared.notify_senders();
                            // println!("skip blocking receiver");
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{self, Rng};
    use sequence::{Owned, Shared};
    use queue::channel;

    #[test]
    fn test_blocking_spsc() {
        const COUNT: u32 = 320000;
        let (mut tx, mut rx) = channel::<Owned, Owned, Blocking, u32>(64);

        let tx = thread::spawn(move|| {
            for i in 0..COUNT {
                tx.send(i).unwrap();
                // println!("sent {}", i);
            }
        });

        let rx = thread::spawn(move|| {
            for i in 0..COUNT {
                assert_eq!(rx.recv(), Some(i));
                // println!("recv {}", i);
            }

            assert!(rx.recv().is_none());
        });

        tx.join().expect("sender thread panicked");
        rx.join().expect("receiver thread panicked");
    }

    #[test]
    fn test_blocking_mpmc() {
        let (tx, mut rx) = channel::<Shared, Shared, Blocking, u32>(16);
        const COUNT: u32 = 1600000;

        drop(thread::spawn(|| {
            let mut tick = 0;
            loop {
                thread::sleep(Duration::from_secs(2));
                tick += 1;
                println!("PING {}", tick);
            }
        }));

        let tx_threads: Vec<_> = (0..4)
            .map(|_n| {
                let mut tx = tx.clone();
                thread::spawn(move|| {
                    let mut rng = rand::thread_rng();
                    let mut acc = 0u64;

                    for _i in 0..COUNT {
                        let num = rng.gen_range(0u32, 64);
                        acc += num as u64;
                        tx.send(num).unwrap();
                        // println!("send {}-{}", _n, _i);
                    }

                    acc
                })
            })
            .collect();

        let rx_threads: Vec<_> = (0..4)
            .map(|_n| {
                let mut rx = rx.clone();
                thread::spawn(move|| {
                    let mut acc = 0u64;

                    for _i in 0..COUNT {
                        acc += rx.recv().unwrap() as u64;
                        // println!("recv {}-{}", _n, _i);
                    }

                    acc
                })
            })
            .collect();

        drop(tx);

        let tx_sum: u64 = tx_threads.into_iter()
            .map(|handle| handle.join().expect("sender thread panicked"))
            .sum();

        let rx_sum: u64 = rx_threads.into_iter()
            .map(|handle| handle.join().expect("receiver thread panicked"))
            .sum();

        assert_eq!(rx.recv(), None);
        assert_eq!(tx_sum, rx_sum);
    }
}
