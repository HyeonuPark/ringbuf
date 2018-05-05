
use std::sync::atomic::{AtomicUsize, AtomicBool};
use std::thread;

use sequence::{Sequence, Limit, Shared};
use buffer::{Buffer, BufInfo};
use role::Role;
use scheduler::{Scheduler, Notify};

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
    scheduler: Scheduler,
}

type Input<T> = <<T as HeadHalf>::Role as Role>::Input;
type Output<T> = <<T as HeadHalf>::Role as Role>::Output;

macro_rules! try_advance {
    ($this:expr, $input:expr, $Role:ty) => (
        match $this.head.seq().try_claim(&mut $this.cache, &$this.head) {
            Some(count) => {
                let buffer = $this.buf.get_ptr(count);
                let res = unsafe {
                    <$Role>::interact(buffer, $input)
                };

                $this.head.seq().commit(&mut $this.cache, count);
                $this.scheduler.pop_blocked::<$Role>();
                Ok(res)
            }
            None => Err($input),
        }
    );
}

impl<B: BufInfo, H: HeadHalf, T: Send> Half<B, H, T> where H::Role: Role<Item=T> {
    pub fn new(
        buf: Buffer<B, T>, head: H, cache: <H::Seq as Sequence>::Cache, scheduler: Scheduler,
    ) -> Self {
        Half {
            buf,
            head,
            cache,
            scheduler,
        }
    }

    pub fn try_advance(&mut self, input: Input<H>) -> Result<Output<H>, Input<H>> {
        try_advance!(self, input, H::Role)
    }

    pub fn sync_advance(&mut self, input: Input<H>) -> Output<H> {
        let mut input = input;

        loop {
            match try_advance!(self, input, H::Role) {
                Ok(output) => return output,
                Err(retry) => input = retry,
            }

            if self.scheduler.register::<H::Role>(Notify::sync()) {
                thread::park();
                self.scheduler.restore();
            } else {
                self.scheduler.pop_blocked::<H::Role>();
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
