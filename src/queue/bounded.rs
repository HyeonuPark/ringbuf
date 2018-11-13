
use sequence::{Sequence, MultiCache};
use buffer::Buffer;

use queue::half::{Half, AdvanceError};
use queue::head::{Head, SenderHead, SenderHalf, ReceiverHead, ReceiverHalf};

#[derive(Debug)]
pub struct Sender<S: Sequence, R: Sequence, T> {
    half: Option<SenderHalf<S, R, T>>,
}

#[derive(Debug)]
pub struct Receiver<S: Sequence, R: Sequence, T> {
    half: Option<ReceiverHalf<S, R, T>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SendError<T> {
    BufferFull(T),
    Closed(T),
}

impl<T> From<AdvanceError<T>> for SendError<T> {
    fn from(e: AdvanceError<T>) -> Self {
        match e {
            AdvanceError::BufferFull(v) => SendError::BufferFull(v),
            AdvanceError::Closed(v) => SendError::Closed(v),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RecvError;

pub fn queue<S, R, T>(capacity: usize) -> (Sender<S, R, T>, Receiver<S, R, T>) where
    S: Sequence, R: Sequence
{
    let head = Head::new(S::default(), R::default());

    let sender = SenderHead::new(head.clone(), capacity);
    let receiver = ReceiverHead::new(head.clone());
    let buffer = Buffer::new(head, capacity);

    // unwrap() is ok for newly created half
    let sender_half = Half::new(buffer.clone(), sender).unwrap();
    let receiver_half = Half::new(buffer, receiver).unwrap();

    let sender = Sender {
        half: Some(sender_half),
    };
    let receiver = Receiver {
        half: Some(receiver_half),
    };

    (sender, receiver)
}

impl<S: Sequence, R: Sequence, T> Sender<S, R, T> {
    pub fn is_closed(&self) -> bool {
        self.half.as_ref().map_or(true, |half| half.is_closed())
    }

    pub fn close(&mut self) {
        self.half.as_mut().map_or((), Half::close)
    }

    pub fn try_send(&mut self, msg: T) -> Result<(), SendError<T>> {
        if let Some(half) = &mut self.half {
            half.try_advance(msg).map_err(SendError::from)
        } else {
            Err(SendError::Closed(msg))
        }
    }
}

impl<S: MultiCache, R: Sequence, T> Clone for Sender<S, R, T> {
    fn clone(&self) -> Self {
        Sender {
            half: self.half.as_ref().and_then(Half::try_clone),
        }
    }
}

impl<S: Sequence, R: Sequence, T> Receiver<S, R, T> {
    pub fn is_closed(&self) -> bool {
        self.half.as_ref().map_or(true, Half::is_closed)
    }

    pub fn close(&mut self) {
        self.half.as_mut().map_or((), Half::close)
    }

    pub fn try_recv(&mut self) -> Result<Option<T>, RecvError> {
        if let Some(half) = &mut self.half {
            match half.try_advance(()) {
                Ok(msg) => Ok(Some(msg)),
                Err(AdvanceError::BufferFull(())) => Err(RecvError),
                Err(AdvanceError::Closed(())) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }
}

impl<S: Sequence, R: MultiCache, T> Clone for Receiver<S, R, T> {
    fn clone(&self) -> Self {
        Receiver {
            half: self.half.as_ref().and_then(Half::try_clone)
        }
    }
}
