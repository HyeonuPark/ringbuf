
use std::sync::atomic::{AtomicUsize, AtomicBool};
use std::thread;

use sequence::{Sequence, Limit, Shared};
use buffer::{Buffer, BufInfo};
use role::{Role};
use scheduler::{Scheduler, Notify};

pub trait HeadHalf: Limit + Clone {
    type Seq: Sequence;
    type Role: Role;

    fn seq(&self) -> &Self::Seq;
    fn role(&self) -> &Self::Role;
    fn count(&self) -> &AtomicUsize;
    fn is_closed(&self) -> &AtomicBool;
}

#[derive(Debug)]
pub struct Half<B: BufInfo, H: HeadHalf, T: Send> where H::Role: Role<Item=T> {
    buf: Buffer<B, T>,
    head: H,
    cache: <H::Seq as Sequence>::Cache,
    scheduler: Scheduler<T>,
}

type Input<T> = <<T as HeadHalf>::Role as Role>::Input;
type Output<T> = <<T as HeadHalf>::Role as Role>::Output;

impl<B: BufInfo, H: HeadHalf, T: Send> Half<B, H, T> where H::Role: Role<Item=T> {
    pub fn new(
        buf: Buffer<B, T>, head: H, cache: <H::Seq as Sequence>::Cache, scheduler: Scheduler<T>,
    ) -> Self {
        Half {
            buf,
            head,
            cache,
            scheduler,
        }
    }

    pub fn try_advance(&mut self, input: Input<H>) -> Result<Output<H>, Input<H>> {
        let head = &self.head;
        let buf = &self.buf;
        let seq = head.seq();
        let cache = &mut self.cache;

        match seq.try_claim(cache, head) {
            Some(count) => {
                let res = unsafe {
                    head.role().exchange_buffer(buf.get_ptr(count), input)
                };
                seq.commit(cache, count);
                Ok(res)
            }
            None => Err(input),
        }
    }

    pub fn sync_advance(&mut self, input: Input<H>) -> Output<H> {
        let head = &self.head;
        let buf = &self.buf;
        let seq = head.seq();
        let cache = &mut self.cache;
        let scheduler = &mut self.scheduler;

        let mut input = input;

        loop {
            // fast path
            if let Some(count) = seq.try_claim(cache, head) {
                let res = unsafe {
                    head.role().exchange_buffer(buf.get_ptr(count), input)
                };
                seq.commit(cache, count);
                return res;
            }

            match scheduler.register(head.role(), input, Notify::sync()) {
                Ok(()) => {
                    thread::park();
                    return scheduler.restore(head.role());
                }
                Err(recover) => input = recover,
            }
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
            scheduler: self.scheduler.clone(),
        }
    }
}
