
use std::marker::PhantomData;
use std::ptr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Sender,
    Receiver,
}

pub trait Role: Default + Copy + private::Sealed {
    type Item;
    type Opposite: Role;
    type Input;
    type Output;

    fn kind() -> Kind;
    unsafe fn interact(buffer: *mut Self::Item, input: Self::Input) -> Self::Output;
}

mod private {
    pub trait Sealed {}
}

#[derive(Debug)]
pub struct Sender<T> {
    _marker: PhantomData<T>,
}

#[derive(Debug)]
pub struct Receiver<T> {
    _marker: PhantomData<T>,
}

impl<T> Role for Sender<T> {
    type Item = T;
    type Opposite = Receiver<T>;
    type Input = T;
    type Output = ();

    fn kind() -> Kind {
        Kind::Sender
    }

    unsafe fn interact(buffer: *mut T, input: T) -> () {
        ptr::write(buffer, input);
    }
}

impl<T> private::Sealed for Sender<T> {}

impl<T> Default for Sender<T> {
    fn default() -> Self {
        Sender {
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender {
            _marker: PhantomData,
        }
    }
}

impl<T> Copy for Sender<T> {}

impl<T> Role for Receiver<T> {
    type Item = T;
    type Opposite = Sender<T>;
    type Input = ();
    type Output = T;

    fn kind() -> Kind {
        Kind::Receiver
    }

    unsafe fn interact(buffer: *mut T, _input: ()) -> T {
        ptr::read(buffer)
    }
}

impl<T> private::Sealed for Receiver<T> {}

impl<T> Default for Receiver<T> {
    fn default() -> Self {
        Receiver {
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Receiver {
            _marker: PhantomData,
        }
    }
}

impl<T> Copy for Receiver<T> {}
