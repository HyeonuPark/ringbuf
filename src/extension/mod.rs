#![allow(missing_docs)]

pub mod blocking;
pub use self::blocking::Blocking;

pub trait Extension: Sync + Sized {
    type Sender;
    type Receiver;

    fn create_triple() -> (Self, Self::Sender, Self::Receiver);
    fn cleanup_sender(&self, local: &mut Self::Sender);
    fn cleanup_receiver(&self, local: &mut Self::Receiver);
}

impl Extension for () {
    type Sender = ();
    type Receiver = ();

    fn create_triple() -> (Self, Self::Sender, Self::Receiver) {
        ((), (), ())
    }
    fn cleanup_sender(&self, _local: &mut ()) {}
    fn cleanup_receiver(&self, _local: &mut ()) {}
}
