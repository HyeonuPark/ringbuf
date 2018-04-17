#![allow(missing_docs)]

pub mod blocking;
pub use self::blocking::Blocking;

pub trait Extension: Sync + Sized {
    type Sender;
    type Receiver;

    fn create_triple() -> (Self, Self::Sender, Self::Receiver);
}

impl Extension for () {
    type Sender = ();
    type Receiver = ();

    fn create_triple() -> (Self, Self::Sender, Self::Receiver) {
        ((), (), ())
    }
}
