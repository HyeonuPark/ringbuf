
use std::sync::Arc;

use role::Role;

mod slot;
mod queue;
mod notify;

use self::queue::{Queue, NodeBox};

pub use self::notify::Notify;
pub use self::slot::Slot;

/// Scheduler for pause/resume executions
/// Just like bounded queue itself,
/// scheduler do not perform heap allocation during pause/resume.
#[derive(Debug)]
pub struct Scheduler {
    queue: Arc<Queue>,
    node: Option<NodeBox>,
    slot: Slot<NodeBox>,
}

unsafe impl Send for Scheduler {}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            queue: Arc::new(Queue::new()),
            node: Some(NodeBox::new()),
            slot: Slot::new(),
        }
    }

    pub fn register<R: Role>(&mut self, notify: Notify) -> bool {
        let node = self.node.take().expect("Scheduler not restored");

        match self.queue.push(R::kind(), notify, node, self.slot.clone()) {
            Ok(()) => true,
            Err(node) => {
                self.node = Some(node);
                false
            }
        }
    }

    pub fn restore(&mut self) {
        assert!(self.node.is_none(), "Scheduler not registered");
        self.node = Some(unsafe { self.slot.read() });
    }

    pub fn pop_blocked<R: Role>(&self) {
        if let Some(notify) = self.queue.pop(R::Opposite::kind()) {
            notify.notify();
        }
    }
}

impl Clone for Scheduler {
    fn clone(&self) -> Self {
        Scheduler {
            queue: self.queue.clone(),
            node: Some(NodeBox::new()),
            slot: Slot::new(),
        }
    }
}
