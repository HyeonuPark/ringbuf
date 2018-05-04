
use std::sync::Arc;

use slot::Slot;
use role::Role;
mod queue;
mod notify;

use self::queue::{Queue, Node};

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

unsafe impl<T: Send> Send for Scheduler<T> {}

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

    pub fn register<R: Role<Item=T>>(
        &mut self, role: &R, input: R::Input, notify: Notify
    ) -> Result<(), R::Input> {
        let mut handle = self.handle.take().expect("Scheduler not restored");
        handle.notify = Some(notify);
        unsafe {
            role.init_slot(&handle.slot, input);
        }

        let mut node = handle.node.take().expect("Node already taken");
        node.init(role.kind(), handle);

        self.queue.push(node).map_err(|mut node| {
            let mut handle = node.take_handle().expect("Handle already taken");
            let recover = unsafe {
                role.recover_from_slot(&handle.slot)
            };
            handle.node = Some(node);
            self.handle = Some(handle);
            recover
        })
    }

    pub fn restore<R: Role<Item=T>>(&mut self, role: &R) -> R::Output {
        assert!(self.handle.is_none(), "Scheduler not consumed");
        let handle = unsafe { Box::from_raw(self.ptr) };
        let res = unsafe {
            role.consume_slot(&handle.slot)
        };
        self.handle = Some(handle);
        res
    }

    pub fn pop_blocked<R: Role<Item=T>>(&self, role: &R, buffer: *mut T) {
        let mut node = match self.queue.pop(role.kind().counterpart()) {
            None => return,
            Some(node) => node,
        };
        let mut handle = node.take_handle().expect("Handle already taken");
        unsafe {
            role.exchange_counterpart(buffer, &handle.slot);
        }
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
