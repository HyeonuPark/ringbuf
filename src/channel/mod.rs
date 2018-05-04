
use std::sync::Arc;

use buffer::Buffer;
use sequence::{Sequence, Shared};
use scheduler::Scheduler;

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

    let sender_head = SenderHead {
        head: head.clone(),
        role: Default::default(),
        capacity,
    };
    let receiver_head = ReceiverHead {
        head: head.clone(),
        role: Default::default(),
    };

    let buf = Buffer::new(head, capacity);
    let scheduler = Scheduler::new();

    let sender_half = Half::new(buf.clone(), sender_head, sender_cache, scheduler.clone());
    let receiver_half = Half::new(buf, receiver_head, receiver_cache, scheduler);

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

    pub fn sync_send(&mut self, msg: T) {
        self.half.sync_advance(msg);
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

    pub fn sync_recv(&mut self) -> T {
        self.half.sync_advance(())
    }
}

impl<S: Sequence, R: Shared, T: Send> Clone for Receiver<S, R, T> {
    fn clone(&self) -> Self {
        Receiver {
            half: self.half.clone(),
        }
    }
}
