
use std::thread::Thread;
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

use intrusive::{Node, Stack};

#[derive(Debug)]
pub struct Blocker {
    // Semantically Option<Box<BlockKind>>
    kind: AtomicPtr<BlockKind>,
}

#[derive(Debug)]
pub enum BlockKind {
    Nothing,
    Sync(Thread),
}

pub type BlockerNode = Arc<Node<Blocker>>;
pub type BlockerContainer = Stack<Blocker>;

impl Blocker {
    pub fn new() -> Arc<Node<Self>> {
        Node::new(Blocker {
            kind: AtomicPtr::new(ptr::null_mut()),
        })
    }

    pub fn container() -> Stack<Self> {
        Stack::new()
    }

    /// Returns true if this blocker isn't hold any `BlockKind` previously.
    pub fn replace(&self, kind: &mut BlockKind) -> bool {
        let prev = self.kind.swap(kind, Ordering::AcqRel);
        prev.is_null()
    }

    pub fn unblock(&self) {
        use self::BlockKind::*;

        let prev = self.kind.swap(ptr::null_mut(), Ordering::AcqRel);

        if let Some(prev) = unsafe { prev.as_ref() } {
            match *prev {
                Nothing => {}
                Sync(ref thread) => {
                    thread.unpark();
                }
            }
        }
    }
}
