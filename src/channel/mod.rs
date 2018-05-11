
use std::sync::Arc;

use buffer::Buffer;
use sequence::{Sequence, Shared};

mod head;
mod half;
mod exchange;

#[cfg(test)]
mod tests;

use self::head::{Head, SenderHead, ReceiverHead};
use self::half::Half;

#[derive(Debug)]
pub struct Sender<S: Sequence, R: Sequence, T: Send> {
    half: Half<Arc<Head<S, R>>, SenderHead<S, R, T>, T>,
}

#[derive(Debug)]
pub struct Receiver<S: Sequence, R: Sequence, T: Send> {
    half: Half<Arc<Head<S, R>>, ReceiverHead<S, R, T>, T>,
}

#[derive(Debug)]
pub struct TrySendError<T>(pub T);

#[derive(Debug)]
pub struct SyncSendError<T>(pub T);

#[derive(Debug)]
pub struct TryRecvError;

#[derive(Debug)]
pub struct SyncRecvError;

pub fn channel<S: Sequence, R: Sequence, T: Send>(
    capacity: usize
) -> (Sender<S, R, T>, Receiver<S, R, T>) {
    let (sender, sender_cache) = S::new();
    let (receiver, receiver_cache) = R::new();

    let head = Arc::new(Head::new(sender, receiver));

    let sender_head = SenderHead::new(head.clone(), capacity);
    let receiver_head = ReceiverHead::new(head.clone());

    let buf = Buffer::new(head, capacity);

    let sender_half = Half::new(buf.clone(), sender_head, sender_cache);
    let receiver_half = Half::new(buf, receiver_head, receiver_cache);

    let sender = Sender {
        half: sender_half,
    };
    let receiver = Receiver {
        half: receiver_half,
    };

    (sender, receiver)
}

impl<S: Sequence, R: Sequence, T: Send> Sender<S, R, T> {
    pub fn try_send(&mut self, msg: T) -> Result<(), TrySendError<T>> {
        self.half.try_advance(msg)
            .map_err(TrySendError)
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
    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        self.half.try_advance(())
            .map_err(|_| TryRecvError)
    }
}

impl<S: Sequence, R: Shared, T: Send> Clone for Receiver<S, R, T> {
    fn clone(&self) -> Self {
        Receiver {
            half: self.half.clone(),
        }
    }
}
