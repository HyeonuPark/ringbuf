
use sequence::{Sequence, Shared};

use super::head::{SenderHead, ReceiverHead, SenderHalf, ReceiverHalf};
use super::half::Half;
use super::chain::Chain;

#[derive(Debug)]
pub struct Sender<S: Sequence, R: Sequence, T: Send> {
    chain: Chain<S, R, T>,
    half: Option<SenderHalf<S, R, T>>,
}

#[derive(Debug)]
pub struct Receiver<S: Sequence, R: Sequence, T: Send> {
    chain: Chain<S, R, T>,
    half: Option<ReceiverHalf<S, R, T>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SendError<T>(pub T);

#[derive(Debug, PartialEq, Eq)]
pub struct ReceiveError;

pub fn queue<S: Sequence, R: Sequence, T: Send>() -> (Sender<S, R, T>, Receiver<S, R, T>) {
    let (sender_chain, receiver_chain) = Chain::new();

    let sender = Sender {
        chain: sender_chain,
        half: None,
    };
    let receiver = Receiver {
        chain: receiver_chain,
        half: None,
    };

    (sender, receiver)
}

impl<S: Sequence, R: Sequence, T: Send> Sender<S, R, T> {
    pub fn is_closed(&self) -> bool {
        self.chain.is_closed()
    }

    pub fn close(&mut self) {
        self.chain.close();
        self.half = None;
    }

    pub fn try_send(&mut self, msg: T) -> Result<(), SendError<T>> {
        let mut msg = msg;

        loop {
            // Sender should operate on the last segment.
            // Invalidate previous `Half` if segment changed.
            if self.chain.move_next() {
                self.half = None;
                self.chain.move_last();
            }

            if self.is_closed() {
                return Err(SendError(msg))
            }

            if let Some(half) = &mut self.half {
                match half.try_advance(msg) {
                    Ok(()) => return Ok(()),
                    Err(msg_back) => {
                        msg = msg_back;
                        // Grow the chain if the buffer is full.
                        self.chain.grow();
                        half.close();
                    }
                }
            }

            // Grow the chain if not yet initialized.
            if self.chain.buf().is_none() {
                self.chain.grow();
            }

            let buf = self.chain.buf().unwrap();
            let head = SenderHead::new(buf.head().clone(), buf.capacity());
            let half = Half::new(buf.clone(), head);

            self.half = Some(half);
        }
    }
}

impl<S: Shared, R: Sequence, T: Send> Clone for Sender<S, R, T> {
    fn clone(&self) -> Self {
        Sender {
            chain: self.chain.clone(),
            half: None,
        }
    }
}

impl<S: Sequence, R: Sequence, T: Send> Receiver<S, R, T> {
    pub fn is_closed(&self) -> bool {
        self.chain.is_closed()
    }

    pub fn close(&mut self) {
        self.chain.close();
        self.half = None;
    }

    pub fn try_recv(&mut self) -> Result<Option<T>, ReceiveError> {
        if self.chain.buf().is_none() && !self.chain.move_next() {
            // send never happened
            return Err(ReceiveError);
        }

        loop {
            if let Some(half) = &mut self.half {
                // Fast path
                if let Ok(msg) = half.try_advance(()) {
                    return Ok(Some(msg))
                }

                if !half.is_closed() {
                    // Queue is open but empty
                    return Err(ReceiveError)
                }

                // It should be checked before moving to next queue segment
                // that this segment is empty
                if let Ok(msg) = half.try_advance(()) {
                    return Ok(Some(msg))
                }

                if !self.chain.move_next() {
                    // The next segment is not exist.
                    // This means either the queue is closed
                    // or the sender is trying to grow the chain.

                    if !self.chain.is_closed() {
                        // Chain is not closed, just wait for sender to complete its task.
                        continue
                    }

                    // Queue is observed to be closed, but there's a chance that
                    // some messagge is sent after our last check.
                    return match half.try_advance(()) {
                        Ok(msg) => Ok(Some(msg)),
                        Err(()) => Ok(None),
                    }
                }
            }

            // `self.half` is invalidated or not exists

            let buf = self.chain.buf().unwrap();
            let head = ReceiverHead::new(buf.head().clone());
            let half = Half::new(buf.clone(), head);

            self.half = Some(half);
        }
    }
}

impl<S: Sequence, R: Shared, T: Send> Clone for Receiver<S, R, T> {
    fn clone(&self) -> Self {
        Receiver {
            chain: self.chain.clone(),
            half: None,
        }
    }
}
