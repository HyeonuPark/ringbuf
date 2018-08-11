
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::ops::Drop;
use std::mem::{transmute_copy, ManuallyDrop};

use role::Role;
use counter::COUNTER_VALID_RANGE;
use buffer::{Buffer, BufRange};
use sequence::{Sequence, Limit, CacheError, CommitError};

pub(crate) trait HeadHalf: Limit + Clone {
    type Seq: Sequence;
    type Role: Role;

    fn seq(&self) -> &Self::Seq;
    fn amount(&self) -> &AtomicUsize;
}

#[derive(Debug)]
pub(crate) struct Half<B, H, T> where
    B: BufRange,
    H: HeadHalf,
    H::Role: Role<Item=T>,
{
    buf: Buffer<B, T>,
    head: H,
    cache: <H::Seq as Sequence>::Cache,
    closed_cache: Cell<bool>,
}

type Input<H> = <<H as HeadHalf>::Role as Role>::Input;
type Output<H> = <<H as HeadHalf>::Role as Role>::Output;

/// Half count per head is limited to half of valid counter range.
/// See docs for `buffer::MAX_BUF_CAPACITY` for more info.
const MAX_HALF_COUNT: usize = COUNTER_VALID_RANGE / 2;

impl<B, H, T> Half<B, H, T> where
    B: BufRange,
    H: HeadHalf,
    H::Role: Role<Item=T>,
{
    pub fn new(buf: Buffer<B, T>, head: H) -> Result<Self, CacheError> {
        let ref_count = head.amount().fetch_add(1, Ordering::Release);
        assert!(ref_count <= MAX_HALF_COUNT,
            "Too many senders or receivers are created for this channel");

        Ok(Half {
            cache: head.seq().cache(&head)?,
            closed_cache: false.into(),
            buf,
            head,
        })
    }

    pub fn try_clone(&self) -> Option<Self> {
        Half::new(self.buf.clone(), self.head.clone()).ok()
    }

    pub fn is_closed(&self) -> bool {
        if self.closed_cache.get() {
            return true;
        }

        if self.head.seq().counter().fetch().is_err() {
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
        self.head.seq().counter().close();
    }
}

impl<B, H, T> Half<B, H, T> where
    B: BufRange,
    H: HeadHalf,
    H::Role: Role<Item=T>,
    T: Send,
{
    pub fn try_advance(&mut self, input: Input<H>) -> Result<Output<H>, Input<H>> {
        if self.closed_cache.get() {
            return Err(input);
        }

        match self.head.seq().claim(&mut self.cache, &self.head) {
            None => Err(input),
            Some(count) => {
                let buffer = self.buf.get(count);
                let (backup, res) = unsafe {(
                    ManuallyDrop::new(transmute_copy::<Input<H>, Input<H>>(&input)),
                    H::Role::interact(buffer, input),
                )};

                match self.head.seq().commit(&mut self.cache, count) {
                    Ok(()) => Ok(res),
                    Err(CommitError) => Err(ManuallyDrop::into_inner(backup)),
                }
            }
        }
    }
}

impl<B, H, T> Drop for Half<B, H, T> where
    B: BufRange,
    H: HeadHalf,
    H::Role: Role<Item=T>,
{
    fn drop(&mut self) {
        let ref_count = self.head.amount().fetch_sub(1, Ordering::Release);

        if ref_count == 1 {
            self.close();
        }
    }
}
