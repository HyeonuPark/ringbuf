
use sequence::{Sequence, MultiCache, CacheError};

use queue::head::{SenderHead, ReceiverHead, SenderHalf, ReceiverHalf};
use queue::half::Half;
use queue::chain::{Chain, pair};

#[derive(Debug)]
pub struct Sender<S, R, T> where
    S: Sequence, R: Sequence, T: Send
{
    chain: Chain<S, R, T>,
    half: Option<SenderHalf<S, R, T>>,
}

#[derive(Debug)]
pub struct Receiver<S, R, T> where
    S: Sequence, R: Sequence, T: Send
{
    chain: Chain<S, R, T>,
    half: Option<ReceiverHalf<S, R, T>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SendError<T>(pub T);

#[derive(Debug, PartialEq, Eq)]
pub struct RecvError;

pub fn queue<S: Sequence, R: Sequence, T: Send>() -> (Sender<S, R, T>, Receiver<S, R, T>) {
    let (sender, receiver) = pair();

    let sender = Sender {
        chain: sender,
        half: None,
    };
    let receiver = Receiver {
        chain: receiver,
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

    pub fn try_send(&mut self, mut msg: T) -> Result<(), SendError<T>> {
        loop {
            // Sender should operate on the last segment of the chain
            // If segment is changed, invalidate previous half.
            if self.chain.goto_next() {
                self.half = None;
                self.chain.goto_last();
            }

            if self.is_closed() {
                return Err(SendError(msg));
            }

            if let Some(half) = &mut self.half {
                match half.try_advance(msg) {
                    Ok(()) => return Ok(()),
                    Err(msg_back) => {
                        msg = msg_back;

                        // Advance can fail in two reasons: segment closed or buffer full.
                        // If buffer is full, just grow the buffer.
                        // If segment is closed, there also are two reasons:
                        // next segment is ready or the entire channel is closed.
                        // In both case .grow() method will handle them correctly.
                        self.chain.grow();
                    }
                }
            }

            // Initialize empty buffer.
            if self.chain.buf().is_none() {
                self.chain.grow();
            }

            // At this point `self.half` is either `None` or invalidated.

            let buf = self.chain.buf().unwrap();
            let head = SenderHead::new(buf.head().clone(), buf.capacity());

            match Half::new(buf.clone(), head) {
                Ok(half) => self.half = Some(half),
                Err(CacheError::SeqClosed) => {
                    // Queue is closed or sender needs to move to next segment.
                    self.half = None;
                    continue
                }
                Err(CacheError::NotAvailable) => {
                    unreachable!("Sender with S: !MultiCache shouldn't be duped")
                }
            }
        }
    }
}

impl<S: MultiCache, R: Sequence, T: Send> Clone for Sender<S, R, T> {
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

    pub fn try_recv(&mut self) -> Result<Option<T>, RecvError> {
        // Nothing ever sent from this channel
        if self.chain.buf().is_none() && !self.chain.goto_next() {
            return Err(RecvError);
        }

        loop {
            // Receiver is not always operate on the last segment of the chain
            // as unreceived messages in closed segment should also be consumed.

            if let Some(half) = &mut self.half {
                // Fast path
                if let Ok(msg) = half.try_advance(()) {
                    return Ok(Some(msg));
                }

                if !half.is_closed() {
                    // Queue is open but empty
                    return Err(RecvError);
                }

                // It should be checked again before leaving this queue segment
                // that this segment is empty.
                if let Ok(msg) = half.try_advance(()) {
                    return Ok(Some(msg));
                }

                if !self.chain.goto_next() {
                    // This segment doesn't have next segment now.
                    // This means either the queue is empty
                    // or some sender is trying to grow the chain.

                    if !self.chain.is_closed() {
                        // Chain is not closed. Just wait for sender to complete its task.
                        continue;
                    }

                    // Queue is observed to be closed, but there's a chance that
                    // some message have been sent after the last check.
                    return match half.try_advance(()) {
                        Ok(msg) => Ok(Some(msg)),
                        Err(()) => Ok(None),
                    };
                }
            }

            // At this point `self.half` is either `None` or invalidated.

            let buf = self.chain.buf().unwrap();
            let head = ReceiverHead::new(buf.head().clone());

            match Half::new(buf.clone(), head) {
                Ok(half) => self.half = Some(half),
                Err(CacheError::SeqClosed) => {
                    // This segment is completly drained out by other receivers.
                    self.half = None;
                    continue
                }
                Err(CacheError::NotAvailable) => {
                    unreachable!("Receiver with R: !MultiCache shouldn't be duped")
                }
            }
        }
    }
}

impl<S: Sequence, R: MultiCache, T: Send> Clone for Receiver<S, R, T> {
    fn clone(&self) -> Self {
        Receiver {
            chain: self.chain.clone(),
            half: None,
        }
    }
}
