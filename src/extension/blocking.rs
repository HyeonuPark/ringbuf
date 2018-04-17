use std::thread::{self, Thread};
use std::sync::{Mutex, mpsc};

use sequence::Sequence;
use queue::{Sender, Receiver};
use extension::Extension;

#[derive(Debug)]
pub struct Blocking {
    sender_rx: Mutex<mpsc::Receiver<Thread>>,
    receiver_rx: Mutex<mpsc::Receiver<Thread>>,
}

#[derive(Debug)]
pub struct SenderLocal {
    tx: mpsc::Sender<Thread>,
}

#[derive(Debug)]
pub struct ReceiverLocal {
    tx: mpsc::Sender<Thread>,
}

impl Blocking {
    fn register_sender(&self, local: &SenderLocal, handle: Thread) {
        local.tx.send(handle).unwrap();
        println!("blocked: sender");
    }

    fn register_receiver(&self, local: &ReceiverLocal, handle: Thread) {
        local.tx.send(handle).unwrap();
        println!("blocked: receiver");
    }

    fn notify_senders(&self) {
        use std::sync::TryLockError as TE;

        match self.sender_rx.try_lock() {
            Ok(handles) => {
                for handle in handles.try_iter() {
                    handle.unpark();
                }
            }
            Err(TE::Poisoned(_)) => {
                panic!("Found poisoned mutex while draining senders")
            }
            Err(TE::WouldBlock) => {}
        }
    }

    fn notify_receivers(&self) {
        use std::sync::TryLockError as TE;

        match self.receiver_rx.try_lock() {
            Ok(handles) => {
                for handle in handles.try_iter() {
                    handle.unpark();
                }
            }
            Err(TE::Poisoned(_)) => {
                panic!("Found poisoned mutex while draining receivers")
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

        (
            Blocking {
                sender_rx: sender_rx.into(),
                receiver_rx: receiver_rx.into(),
            },
            SenderLocal {
                tx: sender_tx,
            },
            ReceiverLocal {
                tx: receiver_tx,
            },
        )
    }

    fn cleanup_sender(&self, _local: &mut SenderLocal) {
        self.notify_receivers();
    }

    fn cleanup_receiver(&self, _local: &mut ReceiverLocal) {
        self.notify_senders();
    }
}

impl<S: Sequence, R: Sequence, T: Send> Sender<S, R, Blocking, T> {
    pub fn send(&mut self, msg: T) -> Result<(), T> {
        use queue::SendErrorKind as K;

        let mut msg = msg;

        loop {
            match self.try_send(msg) {
                Ok(()) => {
                    self.ext_head().notify_receivers();
                    return Ok(());
                }
                Err(err) => match err.kind {
                    K::BufferFull => {
                        self.ext_head().register_sender(self.ext(), thread::current());
                        msg = err.payload;
                        thread::park();
                    }
                    K::ReceiverAllClosed => return Err(err.payload),
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
                    self.ext_head().notify_senders();
                    return msg;
                }
                Err(_) => {
                    self.ext_head().register_receiver(self.ext(), thread::current());
                    thread::park();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use sequence::{Owned};
    use queue::channel;

    #[test]
    fn test_blocking_spsc() {
        let (mut tx, mut rx) = channel::<Owned, Owned, Blocking, u32>(8);

        let (done1, wait) = mpsc::channel();
        let done2 = done1.clone();

        let tx = thread::spawn(move|| {
            for i in 0..6400000 {
                tx.send(i).unwrap();
                println!("sent {}", i);
            }

            done1.send(()).unwrap();
        });

        let rx = thread::spawn(move|| {
            for i in 0..6400000 {
                assert_eq!(rx.recv(), Some(i));
                println!("recv {}", i);
            }

            assert!(rx.recv().is_none());
            done2.send(()).unwrap();
        });

        thread::sleep(Duration::from_secs(5));

        let timeout = "It takes too long!";
        wait.try_recv().expect(timeout);
        wait.try_recv().expect(timeout);

        tx.join().expect("sender thread panicked");
        rx.join().expect("receiver thread panicked");
    }
}
