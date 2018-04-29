
use std::sync::Arc;

use buffer::Buffer;
use sequence::Sequence;

mod half;
use self::half::{Head, Half, SenderHead, ReceiverHead};

#[derive(Debug)]
pub struct Sender<S: Sequence<Item=T>, R: Sequence<Item=T>, T: Send> {
    half: Half<Arc<Head<S, R>>, SenderHead<S, R>, S, T>,
}

#[derive(Debug)]
pub struct Receiver<S: Sequence<Item=T>, R: Sequence<Item=T>, T: Send> {
    half: Half<Arc<Head<S, R>>, ReceiverHead<S, R>, R, T>,
}

#[derive(Debug)]
pub enum TrySendError<T> {
    BufferFull(T),
    ReceiverClosed(T),
}

#[derive(Debug)]
pub struct SendError<T>(pub T);

#[derive(Debug)]
pub struct ReceiveError;

pub fn create<S: Sequence<Item=T>, R: Sequence<Item=T>, T: Send>(
    capacity: usize
) -> (Sender<S, R, T>, Receiver<S, R, T>) {
    let (sender_seq, sender_cache) = S::new();
    let (receiver_seq, receiver_cache) = R::new();

    let head = Arc::new(Head::new(sender_seq, receiver_seq));
    let buf = Buffer::new(head.clone(), capacity);

    let receiver = Receiver {
        half: Half::new(buf.clone(), ReceiverHead(head.clone()), receiver_cache),
    };
    let sender = Sender {
        half: Half::new(buf, SenderHead(head, capacity), sender_cache),
    };

    (sender, receiver)
}

impl<S: Sequence<Item=T>, R: Sequence<Item=T>, T: Send> Sender<S, R, T> {
    pub fn is_closed(&self) -> bool {
        self.half.is_closed()
    }

    pub fn close(&mut self) {
        self.half.close()
    }

    pub fn try_send(&mut self, msg: T) -> Result<(), TrySendError<T>> {
        if let Some(slot) = self.half.try_advance() {
            slot.set(msg);
            return Ok(());
        }

        if self.is_closed() {
            Err(TrySendError::ReceiverClosed(msg))
        } else {
            Err(TrySendError::BufferFull(msg))
        }
    }

    pub fn sync_send(&mut self, msg: T) -> Result<(), SendError<T>> {
        match self.half.sync_advance() {
            Some(slot) => Ok(slot.set(msg)),
            None => Err(SendError(msg)),
        }
    }
}

impl<S: Sequence<Item=T>, R: Sequence<Item=T>, T: Send> Receiver<S, R, T> {
    pub fn is_closed(&self) -> bool {
        self.half.is_closed()
    }

    pub fn close(&mut self) {
        self.half.close()
    }

    pub fn try_recv(&mut self) -> Result<Option<T>, ReceiveError> {
        if let Some(slot) = self.half.try_advance() {
            return Ok(Some(slot.get()));
        }

        if self.is_closed() {
            Ok(None)
        } else {
            Err(ReceiveError)
        }
    }

    pub fn sync_recv(&mut self) -> Option<T> {
        self.half.sync_advance().map(|slot| slot.get())
    }
}
