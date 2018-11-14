
use std::sync::Arc;
use std::collections::LinkedList;
use std::mem;

use role::Kind;

mod atomic_cell;

use self::atomic_cell::AtomicCell;

pub trait Notify {
    /// Resume execution context.
    ///
    /// It should be no-op to notify currently executing context.
    fn notify(self);
}

#[derive(Debug)]
pub struct Queue<T: Notify> {
    remote: Arc<AtomicCell<List<T>>>,
    local: List<T>,
    pocket: Box<List<T>>,
    inbox: Arc<AtomicCell<List<T>>>,
}

#[derive(Debug)]
struct List<T> {
    kind: Option<Kind>,
    list: LinkedList<Msg<T>>,
}

#[derive(Debug)]
struct Msg<T> {
    value: Option<T>,
    inbox: Arc<AtomicCell<List<T>>>,
}

impl<T: Notify> List<T> {
    fn push(&mut self, value: Msg<T>) {
        self.list.push_back(value)
    }

    fn len(&self) -> usize {
        self.list.len()
    }

    fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    fn pop_list(&mut self) -> Option<Self> {
        if self.is_empty() {
            return None
        }

        let tail = self.list.split_off(1);

        Some(List {
            kind: self.kind,
            list: mem::replace(&mut self.list, tail),
        })
    }

    fn notify_all(&mut self, pocket: &mut Box<Self>) {
        while let Some(mut list) = self.pop_list() {
            let inbox = {
                let msg = &mut list.list.front_mut().unwrap();
                msg.value.take().unwrap().notify();
                Arc::clone(&msg.inbox)
            };

            mem::swap(&mut list, pocket);
            inbox.swap(pocket);
            debug_assert!(pocket.is_empty());
            mem::swap(&mut list, pocket);
        }
    }
}

impl<T: Notify> Queue<T> {
    pub fn new() -> Self {
        let mut queue = Queue {
            remote: Arc::default(),
            local: List::default(),
            pocket: Box::default(),
            inbox: Arc::default(),
        };

        queue.local.push(Msg {
            value: None,
            inbox: queue.inbox.clone(),
        });

        queue
    }

    pub fn wait_or_notify(&mut self, value: T, kind: Kind) {
        debug_assert!(self.local.list.len() == 1);
        debug_assert!(self.pocket.is_empty());

        {
            self.local.kind = Some(kind);
            let node = self.local.list.front_mut().unwrap();
            debug_assert!(node.value.is_none());
            debug_assert!(Arc::ptr_eq(&node.inbox, &self.inbox));
            node.value = Some(value);
        }

        self.remote.swap(&mut self.pocket);

        loop {
            let mut to_notify = <(List<T>, List<T>)>::default();

            // merge `self.local` to `self.pocket`
            match (self.local.kind, self.pocket.kind) {
                // contain different kind
                (Some(Kind::Send), Some(Kind::Receive)) |
                (Some(Kind::Receive), Some(Kind::Send)) => {
                    // now self.local cannot be longer than self.pocket
                    if self.local.len() > self.pocket.len() {
                        mem::swap(&mut self.local, &mut *self.pocket);
                    }

                    let remain = List {
                        kind: self.pocket.kind,
                        list: self.pocket.list.split_off(self.local.len()),
                    };
                    let mut pocket = mem::replace(&mut *self.pocket, remain);

                    mem::swap(&mut to_notify.0, &mut self.local);
                    to_notify.1 = pocket;
                }
                // both contain same kind or either one is empty
                _ => {
                    if self.pocket.kind.is_none() {
                        self.pocket.kind = self.local.kind;
                    }

                    self.pocket.list.append(&mut self.local.list);
                }
            }

            self.remote.swap(&mut self.pocket);

            to_notify.0.notify_all(&mut self.pocket);
            to_notify.1.notify_all(&mut self.pocket);

            if self.pocket.is_empty() {
                return
            }
        }
    }
}

impl<T: Notify> Default for Queue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Notify> Clone for Queue<T> {
    fn clone(&self) -> Self {
        Queue {
            remote: self.remote.clone(),
            local: List::default(),
            pocket: Box::default(),
            inbox: Arc::default(),
        }
    }
}

impl<T> Default for List<T> {
    fn default() -> Self {
        List {
            kind: None,
            list: Default::default(),
        }
    }
}
