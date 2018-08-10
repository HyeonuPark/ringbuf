
use sequence::{Sequence, MultiCache};
use buffer::Buffer;

use super::half::Half;
use super::head::{Head, SenderHead, SenderHalf, ReceiverHead, ReceiverHalf};

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
pub struct ReceiveError;

pub fn queue<S, R, T>(capacity: usize) -> (Sender<S, R, T>, Receiver<S, R, T>) where
    S: Sequence, R: Sequence, T: Send
{
    let (sender, receiver) = <(S, R)>::default();
    let head = Head::new(sender, receiver);

    let sender = SenderHead::new(head.clone(), capacity);
    let receiver = ReceiverHead::new(head.clone());
    let buffer = Buffer::new(head, capacity);

    let sender_half = Half::new(buffer.clone(), sender);
    let receiver_half = Half::new(buffer, receiver);

    let sender = Sender {
        half: sender_half.unwrap(), // It's ok for newly created half
    };
    let receiver = Receiver {
        half: receiver_half.unwrap(), // Same as above
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
        self.half.try_advance(msg).map_err(|msg| {
            if self.is_closed() {
                SendError::Closed(msg)
            } else {
                SendError::BufferFull(msg)
            }
        })
    }
}

impl<S: MultiCache, R: Sequence, T: Send> Clone for Sender<S, R, T> {
    fn clone(&self) -> Self {
        Sender {
            half: self.half.try_clone().expect("The channel for this sender is closed"),
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

    pub fn try_recv(&mut self) -> Result<T, ReceiveError> {
        self.half.try_advance(()).map_err(|()| ReceiveError)
    }
}
