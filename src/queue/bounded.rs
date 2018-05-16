
use std::sync::Arc;

use buffer::Buffer;
use sequence::{Sequence, Shared};

use super::head::{Head, SenderHead, ReceiverHead, SenderHalf, ReceiverHalf};
use super::half::Half;

#[derive(Debug)]
pub struct Sender<S: Sequence, R: Sequence, T: Send> {
    half: SenderHalf<S, R, T>,
}

#[derive(Debug)]
pub struct Receiver<S: Sequence, R: Sequence, T: Send> {
    half: ReceiverHalf<S, R, T>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SendError<T> {
    BufferFull(T),
    Closed(T),
}

#[derive(Debug, PartialEq, Eq)]
pub struct RecvError;

pub fn queue<S: Sequence, R: Sequence, T: Send>(
    capacity: usize
) -> (Sender<S, R, T>, Receiver<S, R, T>) {
    let (sender, receiver) = <(S, R)>::default();
    let head = Arc::new(Head::new(sender, receiver));

    let sender_head = SenderHead::new(head.clone(), capacity);
    let receiver_head = ReceiverHead::new(head.clone());

    let buf = Buffer::new(head, capacity);

    let sender_half = Half::new(buf.clone(), sender_head);
    let receiver_half = Half::new(buf, receiver_head);

    let sender = Sender {
        half: sender_half,
    };
    let receiver = Receiver {
        half: receiver_half,
    };

    (sender, receiver)
}

impl<S: Sequence, R: Sequence, T: Send> Sender<S, R, T> {
    pub fn is_closed(&self) -> bool {
        self.half.is_closed()
    }

    pub fn close(&mut self) {
        self.half.close()
    }

    pub fn try_send(&mut self, msg: T) -> Result<(), SendError<T>> {
        if self.half.is_closed() {
            return Err(SendError::Closed(msg));
        }

        self.half.try_advance(msg).map_err(SendError::BufferFull)
    }
}

impl<S: Shared, R: Sequence, T: Send> Clone for Sender<S, R, T> {
    fn clone(&self) -> Self {
        Sender {
            half: self.half.clone(),
        }
    }
}

impl<S: Sequence, R: Sequence, T: Send> Receiver<S, R, T> {
    pub fn is_closed(&self) -> bool {
        self.half.is_closed()
    }

    pub fn close(&mut self) {
        self.half.close()
    }

    pub fn try_recv(&mut self) -> Result<Option<T>, RecvError> {
        match self.half.try_advance(()) {
            Ok(msg) => Ok(Some(msg)),
            Err(()) => {
                if self.half.is_closed() {
                    Ok(None)
                } else {
                    Err(RecvError)
                }
            }
        }
    }
}

impl<S: Sequence, R: Shared, T: Send> Clone for Receiver<S, R, T> {
    fn clone(&self) -> Self {
        Receiver {
            half: self.half.clone(),
        }
    }
}
