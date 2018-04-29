
use std::sync::Arc;
use std::fmt;

use buffer::Buffer;
use sequence::Sequence;

mod half;
use self::half::{Head, Half, SenderHead, ReceiverHead};

#[cfg(test)]
mod tests;

pub struct Sender<S: Sequence, R: Sequence<Item=S::Item>> {
    half: Half<Arc<Head<S, R>>, SenderHead<S, R>, S>,
}

pub struct Receiver<S: Sequence, R: Sequence<Item=S::Item>> {
    half: Half<Arc<Head<S, R>>, ReceiverHead<S, R>, R>,
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

pub fn create<S: Sequence, R: Sequence<Item=S::Item>>(
    capacity: usize
) -> (Sender<S, R>, Receiver<S, R>) where S::Item: Send {
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

impl<S: Sequence, R: Sequence<Item=S::Item>> Sender<S, R> {
    pub fn is_closed(&self) -> bool {
        self.half.is_closed()
    }

    pub fn close(&mut self) {
        self.half.close()
    }

    pub fn try_send(&mut self, msg: S::Item) -> Result<(), TrySendError<S::Item>> {
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

    pub fn sync_send(&mut self, msg: S::Item) -> Result<(), SendError<S::Item>> {
        match self.half.sync_advance() {
            Some(slot) => Ok(slot.set(msg)),
            None => Err(SendError(msg)),
        }
    }
}

impl<S: Sequence, R: Sequence<Item=S::Item>> fmt::Debug for Sender<S, R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Sender {}")
    }
}

impl<S: Sequence, R:Sequence<Item=S::Item>> Receiver<S, R> {
    pub fn is_closed(&self) -> bool {
        self.half.is_closed()
    }

    pub fn close(&mut self) {
        self.half.close()
    }

    pub fn try_recv(&mut self) -> Result<Option<S::Item>, ReceiveError> {
        if let Some(slot) = self.half.try_advance() {
            return Ok(Some(slot.get()));
        }

        if self.is_closed() {
            Ok(None)
        } else {
            Err(ReceiveError)
        }
    }

    pub fn sync_recv(&mut self) -> Option<S::Item> {
        self.half.sync_advance().map(|slot| slot.get())
    }
}

impl<S: Sequence, R: Sequence<Item=S::Item>> fmt::Debug for Receiver<S, R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Receiver {}")
    }
}
