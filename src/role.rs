
use std::marker::PhantomData;
use std::ptr;

use slot::Slot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleKind {
    Sender,
    Receiver,
}

pub trait Role: Default + Copy {
    type Item;
    type Input;
    type Output;

    fn kind(&self) -> RoleKind;
    unsafe fn exchange_buffer(&self, buffer: *mut Self::Item, input: Self::Input) -> Self::Output;
    unsafe fn init_slot(&self, slot: &Slot<Self::Item>, input: Self::Input);
    unsafe fn exchange_counterpart(&self, buffer: *mut Self::Item, slot: &Slot<Self::Item>);
    unsafe fn consume_slot(&self, slot: &Slot<Self::Item>) -> Self::Output;
}

#[derive(Debug)]
pub struct SenderRole<T> {
    _marker: PhantomData<T>,
}

#[derive(Debug)]
pub struct ReceiverRole<T> {
    _marker: PhantomData<T>,
}

impl<T> Role for SenderRole<T> {
    type Item = T;
    type Input = T;
    type Output = ();

    fn kind(&self) -> RoleKind {
        RoleKind::Sender
    }

    unsafe fn exchange_buffer(&self, buffer: *mut T, input: T) -> () {
        ptr::write(buffer, input);
    }

    unsafe fn init_slot(&self, slot: &Slot<T>, input: T) {
        slot.write(input);
    }

    unsafe fn exchange_counterpart(&self, buffer: *mut T, slot: &Slot<T>) {
        slot.read_from(buffer);
    }

    unsafe fn consume_slot(&self, _slot: &Slot<T>) -> () {
        // no-op
    }
}

impl<T> Default for SenderRole<T> {
    fn default() -> Self {
        SenderRole {
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for SenderRole<T> {
    fn clone(&self) -> Self {
        SenderRole {
            _marker: PhantomData,
        }
    }
}

impl<T> Copy for SenderRole<T> {}

impl<T> Role for ReceiverRole<T> {
    type Item = T;
    type Input = ();
    type Output = T;

    fn kind(&self) -> RoleKind {
        RoleKind::Receiver
    }

    unsafe fn exchange_buffer(&self, buffer: *mut T, _input: ()) -> T {
        ptr::read(buffer)
    }

    unsafe fn init_slot(&self, _slot: &Slot<T>, _input: ()) {
        // no-op
    }

    unsafe fn exchange_counterpart(&self, buffer: *mut T, slot: &Slot<T>) {
        slot.write_to(buffer);
    }

    unsafe fn consume_slot(&self, slot: &Slot<T>) -> T {
        slot.read()
    }
}

impl<T> Default for ReceiverRole<T> {
    fn default() -> Self {
        ReceiverRole {
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for ReceiverRole<T> {
    fn clone(&self) -> Self {
        ReceiverRole {
            _marker: PhantomData,
        }
    }
}

impl<T> Copy for ReceiverRole<T> {}
