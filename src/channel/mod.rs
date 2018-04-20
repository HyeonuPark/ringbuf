//! Thread safe generalized queue implementation
//! using [`RingBuf`](../ringbuf/struct.RingBuf.html)
//! and [`Sequence`](../sequence/trait.Sequence.html).

use std::sync::atomic::{AtomicUsize, AtomicBool};
use ringbuf::{RingBuf, BufInfo};
use sequence::Sequence;
use counter::Counter;

mod sender;
mod receiver;

pub use self::sender::{Sender, SendError, SendErrorKind};
pub use self::receiver::{Receiver, ReceiveError};

#[derive(Debug)]
struct Head<S: Sequence, R: Sequence, E: Default> {
    sender: S,
    receiver: R,
    is_closed: AtomicBool,
    senders_count: AtomicUsize,
    receivers_count: AtomicUsize,
    extension: E,
}

impl<S: Sequence, R: Sequence, E: Default> BufInfo for Head<S, R, E> {
    fn start(&self) -> Counter {
        self.receiver.count()
    }

    fn end(&self) -> Counter {
        self.sender.count()
    }
}

/// Creates a bounded channel for communicating between asynchronous tasks.
pub fn channel<S, R, E, T>(
    capacity: usize
) -> (Sender<S, R, E, T>, Receiver<S, R, E, T>) where
    S: Sequence,
    R: Sequence,
    E: Default,
    T: Send,
{
    let (sender, receiver, sender_cache, receiver_cache, extension) = Default::default();

    let head = Head {
        sender,
        receiver,
        is_closed: AtomicBool::new(false),
        senders_count: AtomicUsize::new(1),
        receivers_count: AtomicUsize::new(1),
        extension,
    };

    let buf = RingBuf::new(head, capacity);

    (
        Sender::new(buf.clone(), sender_cache),
        Receiver::new(buf, receiver_cache),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use rand::{thread_rng, Rng};
    use sequence::{Owned, Shared};

    #[test]
    fn test_spining_spsc() {
        const COUNT: u32 = 64000;
        let (mut tx, mut rx) = channel::<Owned, Owned, (), u32>(16);

        let tx = thread::spawn(move|| {
            for i in 0..COUNT {
                loop {
                    if let Ok(()) = tx.try_send(i) {
                        break;
                    }
                }
            }
        });

        let rx = thread::spawn(move|| {
            for i in 0..COUNT {
                loop {
                    if let Ok(recv) = rx.try_recv() {
                        assert_eq!(i, recv.unwrap());
                        break;
                    }
                }
            }
        });

        tx.join().expect("Sender thread panicked");
        rx.join().expect("Receiver thread panicked");
    }

    #[test]
    fn test_spninning_mpmc() {
        const COUNT: u32 = 2000;
        let (tx, mut rx) = channel::<Shared, Shared, (), u32>(64);

        let tx_handles: Vec<_> = (0..4).map(|_n| {
            let mut tx = tx.clone();
            thread::spawn(move|| {
                let mut rng = thread_rng();
                let mut acc = 0u64;

                for _i in 0..COUNT {
                    loop {
                        let v: u32 = rng.gen();
                        if let Ok(()) = tx.try_send(v) {
                            // println!("sent: {} - {}", _n, _i);
                            acc += v as u64;
                            break;
                        }
                    }
                }

                acc
            })
        }).collect();

        let rx_handles: Vec<_> = (0..4).map(|_n| {
            let mut rx = rx.clone();
            thread::spawn(move|| {
                let mut acc = 0u64;

                for _i in 0..COUNT {
                    loop {
                        if let Ok(v) = rx.try_recv() {
                            // println!("recv: {} - {}", _n, _i);
                            acc += v.unwrap() as u64;
                            break;
                        }
                    }
                }

                acc
            })
        }).collect();

        drop(tx);

        let tx_acc: u64 = tx_handles.into_iter()
            .map(|h| h.join().expect("Sender thread panicked"))
            .sum();

        let rx_acc: u64 = rx_handles.into_iter()
            .map(|h| h.join().expect("Receiver thread panicked"))
            .sum();

        assert_eq!(tx_acc, rx_acc);
        assert_eq!(rx.try_recv().unwrap(), None);
    }
}
