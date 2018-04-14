

pub mod counter;
pub mod sequence;
pub mod ringbuf;
pub mod channel;

pub mod spsc {
    use sequence::Owned;
    use channel as chan;

    pub use channel::{SendError, SendErrorKind, ReceiveError};
    pub type Sender<T> = chan::Sender<Owned, Owned, T>;
    pub type Receiver<T> = chan::Receiver<Owned, Owned, T>;

    #[inline]
    pub fn channel<T: Send>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        chan::channel(capacity)
    }
}

pub mod mpsc {
    use sequence::{Owned, Shared};
    use channel as chan;

    pub use channel::{SendError, SendErrorKind, ReceiveError};
    pub type Sender<T> = chan::Sender<Shared, Owned, T>;
    pub type Receiver<T> = chan::Receiver<Shared, Owned, T>;

    #[inline]
    pub fn channel<T: Send>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        chan::channel(capacity)
    }
}

pub mod spmc {
    use sequence::{Owned, Shared};
    use channel as chan;

    pub use channel::{SendError, SendErrorKind, ReceiveError};
    pub type Sender<T> = chan::Sender<Owned, Shared, T>;
    pub type Receiver<T> = chan::Receiver<Owned, Shared, T>;

    #[inline]
    pub fn channel<T: Send>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        chan::channel(capacity)
    }
}

pub mod mpmc {
    use sequence::Shared;
    use channel as chan;

    pub use channel::{SendError, SendErrorKind, ReceiveError};
    pub type Sender<T> = chan::Sender<Shared, Shared, T>;
    pub type Receiver<T> = chan::Receiver<Shared, Shared, T>;

    #[inline]
    pub fn channel<T: Send>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        chan::channel(capacity)
    }
}
