
use std::thread::{self, Thread};

#[derive(Debug)]
pub struct Notify {
    kind: NotifyKind,
}

#[derive(Debug)]
enum NotifyKind {
    Sync(Thread),
}

use self::NotifyKind::*;

impl Notify {
    pub fn notify(self) {
        match self.kind {
            Sync(thread) => thread.unpark(),
        }
    }

    pub fn sync() -> Self {
        Notify {
            kind: Sync(thread::current())
        }
    }
}
