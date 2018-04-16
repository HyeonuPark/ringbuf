use std::thread::{self, Thread};
use std::marker::PhantomData;

use sequence::Sequence;
use channel::{Sender, Receiver};
use extension::Extension;

pub struct Blocking<S: Sequence, R: Sequence> {
    _marker: PhantomData<(S, R)>,
}

unsafe impl<S: Sequence, R: Sequence> Send for Blocking<S, R> {}
unsafe impl<S: Sequence, R: Sequence> Sync for Blocking<S, R> {}

pub struct SenderLocal<S: Sequence, R: Sequence> {
    sender_tx: Sender<S, R, (), Thread>,
    receiver_rx: Receiver<R, S, (), Thread>,
}

pub struct ReceiverLocal<S: Sequence, R: Sequence> {
    receiver_tx: Sender<R, S, (), Thread>,
    sender_rx: Receiver<S, R, (), Thread>,
}

impl<S: Sequence, R: Sequence> Default for Blocking<S, R> {
    fn default() -> Self {
        Blocking {
            _marker: PhantomData,
        }
    }
}

impl<S: Sequence, R: Sequence> Extension for Blocking<S, R> {
    type Sender = SenderLocal<S, R>;
    type Receiver = ReceiverLocal<S, R>;

    fn create_triple() -> (Self, Self::Sender, Self::Receiver) {
        unimplemented!()
    }

    fn cleanup_sender(&self, local: &mut Self::Sender) {
        unimplemented!()
    }

    fn cleanup_receiver(&self, local: &mut Self::Receiver) {
        unimplemented!()
    }
}
