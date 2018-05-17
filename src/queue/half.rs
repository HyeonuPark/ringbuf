
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::cell::Cell;
use std::ops::Drop;

use sequence::{Sequence, Limit, Shared};
use buffer::{Buffer, BufInfo};
use role::Role;

pub trait HeadHalf: Limit + Clone {
    type Seq: Sequence;
    type Role: Role;

    fn seq(&self) -> &Self::Seq;
    fn amount(&self) -> &AtomicUsize;
    fn is_closed(&self) -> &AtomicBool;
}

#[derive(Debug)]
pub struct Half<B: BufInfo, H: HeadHalf, T: Send> where H::Role: Role<Item=T> {
    buf: Buffer<B, T>,
    head: H,
    cache: <H::Seq as Sequence>::Cache,
    closed_cache: Cell<bool>,
}

type Input<T> = <<T as HeadHalf>::Role as Role>::Input;
type Output<T> = <<T as HeadHalf>::Role as Role>::Output;

impl<B: BufInfo, H: HeadHalf, T: Send> Half<B, H, T> where H::Role: Role<Item=T> {
    pub fn new(
        buf: Buffer<B, T>, head: H
    ) -> Self {
        head.amount().fetch_add(1, Ordering::Release);

        Half {
            cache: head.seq().cache(&head),
            closed_cache: false.into(),
            buf,
            head,
        }
    }

    pub fn is_closed(&self) -> bool {
        if self.closed_cache.get() {
            return true;
        }

        if self.head.is_closed().load(Ordering::Acquire) {
            self.closed_cache.set(true);
            return true;
        }

        false
    }

    pub fn close(&mut self) {
        if self.closed_cache.get() {
            return;
        }

        self.closed_cache.set(true);
        self.head.is_closed().store(true, Ordering::Release);
    }

    pub fn try_advance(&mut self, input: Input<H>) -> Result<Output<H>, Input<H>> {
        match self.head.seq().try_claim(&mut self.cache, &self.head) {
            Some(count) => {
                let buffer = self.buf.get_ptr(count);
                let res = unsafe {
                    H::Role::interact(buffer, input)
                };

                self.head.seq().commit(&mut self.cache, count);
                Ok(res)
            }
            None => Err(input),
        }
    }
}

impl<B, H, T> Clone for Half<B, H, T> where
    B: BufInfo,
    H: HeadHalf,
    H::Seq: Shared,
    H::Role: Role<Item=T>,
    T: Send,
{
    fn clone(&self) -> Self {
        self.head.amount().fetch_add(1, Ordering::Relaxed);

        Half {
            buf: self.buf.clone(),
            head: self.head.clone(),
            cache: self.head.seq().cache(&self.head),
            closed_cache: self.closed_cache.clone(),
        }
    }
}

impl<B, H, T> Drop for Half<B, H, T> where
    B: BufInfo,
    H: HeadHalf,
    H::Role: Role<Item=T>,
    T: Send,
{
    fn drop(&mut self) {
        let ref_count = self.head.amount().fetch_sub(1, Ordering::Release);

        match ref_count {
            1 => self.close(),
            0 => panic!("RefCount is overflowed. Really?!"),
            _ => {}
        }
    }
}
