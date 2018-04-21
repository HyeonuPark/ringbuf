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
                    (BlockReceiver, _) |
                    (WakeReceiver, WakeReceiver) => break,

                    (WakeReceiver, BlockReceiver) => {
                        next_cond = BlockReceiver;
                        next_target = WakeReceiver;
                    }

                    (_, _) => unreachable!(),
                }
            }

            #[inline(always)]
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

                            (Neutral, next @ BlockSender) |
                            (Neutral, next @ WakeSender) |
                            (BlockSender, next @ Neutral) |
                            (BlockSender, next @ WakeSender) |
                            (WakeSender, next @ Neutral) |
                            (WakeSender, next @ BlockSender) => {
                                next_cond = next;
                            }

                            (Locked, _) |
                            (BlockReceiver, _) |
                            (WakeReceiver, _) => unreachable!(),
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
                    (BlockSender, _) |
                    (WakeSender, WakeSender) => break,

                    (WakeSender, BlockSender) => {
                        next_cond = BlockSender;
                        next_target = WakeSender;
                    }

                    (_, _) => unreachable!(),
                }
            }

            #[inline(always)]
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

                            (Neutral, next @ BlockReceiver) |
                            (Neutral, next @ WakeReceiver) |
                            (BlockReceiver, next @ Neutral) |
                            (BlockReceiver, next @ WakeReceiver) |
                            (WakeReceiver, next @ Neutral) |
                            (WakeReceiver, next @ BlockReceiver) => {
                                next_cond = next;
                            }

                            (Locked, _) |
                            (BlockSender, _) |
                            (WakeSender, _) => unreachable!(),
                        }
                    }
                }
            }
        }
    }
}
