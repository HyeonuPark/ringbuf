
use std::sync::atomic::{AtomicUsize, AtomicBool};

use sequence::{Sequence, Limit, Shared};
use buffer::{Buffer, BufInfo};
use role::Role;

pub trait HeadHalf: Limit + Clone {
    type Seq: Sequence;
    type Role: Role;

    fn seq(&self) -> &Self::Seq;
    fn count(&self) -> &AtomicUsize;
    fn is_closed(&self) -> &AtomicBool;
}

#[derive(Debug)]
pub struct Half<B: BufInfo, H: HeadHalf, T: Send> where H::Role: Role<Item=T> {
    buf: Buffer<B, T>,
    head: H,
    cache: <H::Seq as Sequence>::Cache,
}

type Input<T> = <<T as HeadHalf>::Role as Role>::Input;
type Output<T> = <<T as HeadHalf>::Role as Role>::Output;

impl<B: BufInfo, H: HeadHalf, T: Send> Half<B, H, T> where H::Role: Role<Item=T> {
    pub fn new(
        buf: Buffer<B, T>, head: H, cache: <H::Seq as Sequence>::Cache
    ) -> Self {
        Half {
            buf,
            head,
            cache,
        }
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
        Half {
            buf: self.buf.clone(),
            head: self.head.clone(),
            cache: self.head.seq().new_cache(&self.head),
        }
    }
}
