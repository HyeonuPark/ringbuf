
use std::sync::atomic::{AtomicUsize, AtomicBool};

use sequence::{Sequence, Limit, Shared};
use buffer::{Buffer, BufInfo};

pub trait HeadHalf: Limit + Clone {
    type Seq: Sequence;

    fn seq(&self) -> &Self::Seq;
    fn count(&self) -> &AtomicUsize;
    fn is_closed(&self) -> &AtomicBool;
}

#[derive(Debug)]
pub struct Half<B: BufInfo, H: HeadHalf, T: Send> {
    buf: Buffer<B, T>,
    head: H,
    cache: <H::Seq as Sequence>::Cache,
}

impl<B: BufInfo, H: HeadHalf, T: Send> Half<B, H, T> {
    pub fn new(buf: Buffer<B, T>, head: H, cache: <H::Seq as Sequence>::Cache) -> Self {
        Half {
            buf,
            head,
            cache,
        }
    }

    pub fn try_advance<F: FnOnce(*mut T, V) -> U, U, V>(&mut self, f: F, arg: V) -> Result<U, V> {
        let head = &self.head;
        let buf = &self.buf;
        let seq = head.seq();
        let cache = &mut self.cache;

        match seq.try_claim(cache, head) {
            Some(count) => {
                let res = f(buf.get_ptr(count), arg);
                seq.commit(cache, count);
                Ok(res)
            }
            None => Err(arg),
        }
    }
}

impl<B, H, T> Clone for Half<B, H, T> where
    B: BufInfo,
    H: HeadHalf,
    H::Seq: Shared,
    T: Send
{
    fn clone(&self) -> Self {
        Half {
            buf: self.buf.clone(),
            head: self.head.clone(),
            cache: self.head.seq().new_cache(&self.head),
        }
    }
}
