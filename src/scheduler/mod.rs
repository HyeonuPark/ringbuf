
use std::sync::Arc;

mod slot;
mod queue;
mod atomic;
mod notify;

use self::queue::{Queue, Node};

pub use self::slot::Slot;
pub use self::notify::Notify;

/// Scheduler for pause/resume executions
/// Just like bounded queue itself,
/// scheduler do not perform heap allocation during pause/resume.
#[derive(Debug)]
pub struct Scheduler<T> {
    queue: Arc<Queue<T>>,
    handle: Option<Box<Handle<T>>>,
    ptr: *mut Handle<T>,
}

#[derive(Debug)]
pub struct Handle<T> {
    slot: Slot<T>,
    notify: Option<Notify>,
    node: Option<Box<Node<T>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Sender,
    Receiver,
}

impl<T> Handle<T> {
    fn new() -> Box<Self> {
        Box::new(Handle {
            slot: Slot::new(),
            notify: None,
            node: Some(Node::new()),
        })
    }
}

impl<T> Scheduler<T> {
    pub fn new() -> Self {
        let mut handle = Handle::new();

        Scheduler {
            queue: Arc::new(Queue::new()),
            ptr: &mut *handle,
            handle: Some(handle),
        }
    }

    pub fn slot(&self) -> Option<&Slot<T>> {
        self.handle.as_ref().map(|handle| &handle.slot)
    }

    pub fn register(&mut self, role: Role, notify: Notify) -> bool {
        let mut handle = self.handle.take().expect("Scheduler not restored");
        handle.notify = Some(notify);

        let mut node = handle.node.take().expect("Node already taken");
        node.init(role, handle);

        match self.queue.push(node) {
            Ok(()) => true,
            Err(mut node) => {
                let mut handle = node.take_handle().expect("Handle already taken");
                handle.node = Some(node);
                self.handle = Some(handle);
                false
            }
        }
    }

    pub fn restore(&mut self) {
        assert!(self.handle.is_none(), "Scheduler not consumed");
        self.handle = Some(unsafe { Box::from_raw(self.ptr) });
    }

    pub fn pop_blocked<F: FnOnce(&Slot<T>)>(&self, role: Role, cb: F) {
        let mut node = match self.queue.pop(role) {
            None => return,
            Some(node) => node,
        };
        let mut handle = node.take_handle().expect("Handle already taken");
        cb(&handle.slot);
        handle.node = Some(node);
        handle.notify.take().unwrap().notify();
        Box::into_raw(handle);
    }
}

impl<T> Clone for Scheduler<T> {
    fn clone(&self) -> Self {
        let mut handle = Handle::new();

        Scheduler {
            queue: self.queue.clone(),
            ptr: &mut *handle,
            handle: Some(handle),
        }
    }
}
