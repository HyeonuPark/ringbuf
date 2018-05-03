
use std::sync::Arc;
use std::ptr;

use buffer::Buffer;
use sequence::{Sequence, Shared};

mod head;
mod half;

use self::head::{Head, SenderHead, ReceiverHead};
use self::half::Half;

#[derive(Debug)]
pub struct Sender<S: Sequence, R: Sequence, T: Send> {
    half: Half<Arc<Head<S, R>>, SenderHead<S, R>, T>,
}

#[derive(Debug)]
pub struct Receiver<S: Sequence, R: Sequence, T: Send> {
    half: Half<Arc<Head<S, R>>, ReceiverHead<S, R>, T>,
}

#[derive(Debug)]
pub struct TrySendError<T>(pub T);

#[derive(Debug)]
pub struct TryRecvError;

pub fn channel<S: Sequence, R: Sequence, T: Send>(
    capacity: usize
) -> (Sender<S, R, T>, Receiver<S, R, T>) {
    let (sender, sender_cache) = S::new();
    let (receiver, receiver_cache) = R::new();

    let head = Arc::new(Head::new(sender, receiver));

    let sender_head = SenderHead {
        head: head.clone(),
        capacity,
    };
    let receiver_head = ReceiverHead {
        head: head.clone(),
    };

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
        self.half.try_advance(|slot, msg| unsafe { ptr::write(slot, msg); }, msg)
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
        self.half.try_advance(|slot, _| unsafe { ptr::read(slot) }, ())
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
