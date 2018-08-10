
use std::marker::PhantomData;
use std::ptr;

pub trait Role: private::Sealed {
    type Item;
    type Input;
    type Output;

    unsafe fn interact(target: *mut Self::Item, input: Self::Input) -> Self::Output;
}

#[derive(Debug)]
pub struct Send<T> {
    _marker: PhantomData<T>,
}

#[derive(Debug)]
pub struct Receive<T> {
    _marker: PhantomData<T>,
}

impl<T> Role for Send<T> {
    type Item = T;
    type Input = T;
    type Output = ();

    unsafe fn interact(target: *mut T, input: T) {
        ptr::write(target, input);
    }
}

impl<T> Role for Receive<T> {
    type Item = T;
    type Input = ();
    type Output = T;

    unsafe fn interact(target: *mut T, _: ()) -> T {
        ptr::read(target)
    }
}

impl<T> private::Sealed for Send<T> {}
impl<T> private::Sealed for Receive<T> {}

mod private {
    pub trait Sealed {}
}
