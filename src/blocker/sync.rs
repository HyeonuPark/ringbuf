use std::thread::{self, Thread};

use crossbeam::sync::MsQueue;

use sequence::Sequence;
use channel::{Sender, Receiver, SendError};

use super::state::{State, AtomicState};
use self::State::*;

#[derive(Debug)]
pub struct Blocker {
    senders: MsQueue<Thread>,
    receivers: MsQueue<Thread>,
    state: AtomicState,
}

impl Default for Blocker {
    fn default() -> Self {
        Blocker {
            senders: MsQueue::new(),
            receivers: MsQueue::new(),
            state: AtomicState::new(),
        }
    }
}

impl<S: Sequence, R: Sequence, T: Send> Sender<S, R, Blocker, T> {
    pub fn send(&mut self, msg: T) {
        let mut msg = msg;

        loop {
            let res = self.try_send(msg);
            let ext = self.ext();

            let (mut next_cond, mut next_target) = (BlockReceiver, WakeReceiver);
            loop {
                match (next_cond, ext.state.transit(next_cond, next_target)) {
                    (_, Locked) => {}
                    (BlockReceiver, BlockReceiver) => {
                        while let Some(blocked) = ext.receivers.try_pop() {
                            blocked.unpark();
                        }

                        next_cond = WakeReceiver;
                        next_target = Neutral;
                    }

                    (BlockReceiver, Neutral)      |
                    (BlockReceiver, BlockSender)  |
                    (BlockReceiver, WakeSender)   |
                    (BlockReceiver, WakeReceiver) |

                    (WakeReceiver, Neutral)     |
                    (WakeReceiver, BlockSender) |
                    (WakeReceiver, WakeSender)  |
                    (WakeReceiver, WakeReceiver) => break,

                    (WakeReceiver, BlockReceiver) => {
                        next_cond = BlockReceiver;
                        next_target = WakeReceiver;
                    }

                    (Neutral,     prev @ _) |
                    (Locked,      prev @ _) |
                    (BlockSender, prev @ _) |
                    (WakeSender,  prev @ _) => unreachable!("{:?}", prev),
                }
            }

            fn push_current(ext: &Blocker) {
                ext.senders.push(thread::current());
                let locked = ext.state.transit(Locked, BlockSender);
                debug_assert_eq!(locked, Locked);
                thread::park();
            }

            match res {
                Ok(()) => return,
                Err(SendError(msg_back)) => {
                    msg = msg_back;

                    let mut next_cond = Neutral;
                    loop {
                        match (next_cond, ext.state.transit(next_cond, Locked)) {
                            (_, Locked) => {}
                            (_, BlockReceiver) |
                            (_, WakeReceiver) => break,

                            (Neutral, Neutral) |
                            (BlockSender, BlockSender) |
                            (WakeSender, WakeSender) => {
                                push_current(ext);
                                break;
                            }

                            (Neutral,     next @ BlockSender) |
                            (Neutral,     next @ WakeSender)  |
                            (BlockSender, next @ Neutral)     |
                            (BlockSender, next @ WakeSender)  |
                            (WakeSender,  next @ Neutral)     |
                            (WakeSender,  next @ BlockSender) => {
                                next_cond = next;
                            }

                            (Locked,        prev @ _) |
                            (BlockReceiver, prev @ _) |
                            (WakeReceiver,  prev @ _) => unreachable!("{:?}", prev),
                        }
                    }
                }
            }
        }
    }
}

impl<S: Sequence, R: Sequence, T: Send> Receiver<S, R, Blocker, T> {
    pub fn recv(&mut self) -> T {
        loop {
            let res = self.try_recv();
            let ext = self.ext();

            let (mut next_cond, mut next_target) = (BlockSender, WakeSender);
            loop {
                match (next_cond, ext.state.transit(next_cond, next_target)) {
                    (_, Locked) => {}
                    (BlockSender, BlockSender) => {
                        while let Some(blocked) = ext.senders.try_pop() {
                            blocked.unpark();
                        }

                        next_cond = WakeSender;
                        next_target = Neutral;
                    }

                    (BlockSender, Neutral)       |
                    (BlockSender, BlockReceiver) |
                    (BlockSender, WakeReceiver)  |
                    (BlockSender, WakeSender)    |

                    (WakeSender, Neutral)       |
                    (WakeSender, BlockReceiver) |
                    (WakeSender, WakeReceiver)  |
                    (WakeSender, WakeSender) => break,

                    (WakeSender, BlockSender) => {
                        next_cond = BlockSender;
                        next_target = WakeSender;
                    }

                    (Neutral,       prev @ _) |
                    (Locked,        prev @ _) |
                    (BlockReceiver, prev @ _) |
                    (WakeReceiver,  prev @ _) => unreachable!("{:?}", prev),
                }
            }

            fn push_current(ext: &Blocker) {
                ext.receivers.push(thread::current());
                let locked = ext.state.transit(Locked, BlockReceiver);
                debug_assert_eq!(locked, Locked);
                thread::park();
            }

            match res {
                Some(msg) => return msg,
                None => {
                    let mut next_cond = Neutral;
                    loop {
                        match (next_cond, ext.state.transit(next_cond, Locked)) {
                            (_, Locked) => {}
                            (_, BlockSender) |
                            (_, WakeSender) => break,

                            (Neutral, Neutral) |
                            (BlockReceiver, BlockReceiver) |
                            (WakeReceiver, WakeReceiver) => {
                                push_current(ext);
                                break;
                            }

                            (Neutral,       next @ BlockReceiver) |
                            (Neutral,       next @ WakeReceiver)  |
                            (BlockReceiver, next @ Neutral)       |
                            (BlockReceiver, next @ WakeReceiver)  |
                            (WakeReceiver,  next @ Neutral)       |
                            (WakeReceiver,  next @ BlockReceiver) => {
                                next_cond = next;
                            }

                            (Locked,      prev @ _) |
                            (BlockSender, prev @ _) |
                            (WakeSender,  prev @ _) => unreachable!("{:?}", prev),
                        }
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
    use channel::channel;

    #[test]
    fn test_blocking_spsc() {
        const COUNT: u32 = 320000;
        let (mut tx, mut rx) = channel::<Owned, Owned, Blocker, u32>(64);

        let tx = thread::spawn(move|| {
            for i in 0..COUNT {
                tx.send(i);
                // println!("sent {}", i);
            }
        });

        let rx = thread::spawn(move|| {
            for i in 0..COUNT {
                assert_eq!(rx.recv(), i);
                // println!("recv {}", i);
            }
        });

        tx.join().expect("sender thread panicked");
        rx.join().expect("receiver thread panicked");
    }

    #[test]
    fn test_blocking_mpmc() {
        let (tx, rx) = channel::<Shared, Shared, Blocker, u32>(16);
        const COUNT: u32 = 16000;

        let tx_threads: Vec<_> = (0..8)
            .map(|_n| {
                let mut tx = tx.clone();
                thread::Builder::new()
                    .name(format!("mpmc-sender-{}", _n))
                    .spawn(move|| {
                        let mut rng = rand::thread_rng();
                        let mut acc = 0u64;

                        for _i in 0..COUNT {
                            let num = rng.gen_range(0u32, 256);
                            acc += num as u64;
                            tx.send(num);
                            // println!("send {}-{}", _n, _i);
                        }

                        acc
                    })
                    .unwrap()
            })
            .collect();

        let rx_threads: Vec<_> = (0..8)
            .map(|_n| {
                let mut rx = rx.clone();
                thread::Builder::new()
                    .name(format!("mpmc-receiver-{}", _n))
                    .spawn(move|| {
                        let mut acc = 0u64;

                        for _i in 0..COUNT {
                            acc += rx.recv() as u64;
                            // println!("recv {}-{}", _n, _i);
                        }

                        acc
                    })
                    .unwrap()
            })
            .collect();

        drop(tx);

        let tx_sum: u64 = tx_threads.into_iter()
            .map(|handle| handle.join().expect("sender thread panicked"))
            .sum();

        let rx_sum: u64 = rx_threads.into_iter()
            .map(|handle| handle.join().expect("receiver thread panicked"))
            .sum();

        assert_eq!(tx_sum, rx_sum);
    }
}
