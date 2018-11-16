
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
    fn new(inbox: Arc<AtomicCell<Self>>) -> Self {
        let mut list = LinkedList::new();
        list.push_front(Msg {
            value: None,
            inbox,
        });

        List {
            kind: None,
            list,
        }
    }

    fn empty() -> Self {
        List {
            kind: None,
            list: LinkedList::new(),
        }
    }

    /// Assume this list has single node with `None` value, and init.
    fn init(&mut self, kind: Kind, value: T) {
        debug_assert!(self.kind.is_none());
        debug_assert!(self.list.len() == 1);

        let msg = self.list.front_mut().unwrap();
        debug_assert!(msg.value.is_none());

        self.kind = Some(kind);
        msg.value = Some(value);
    }

    fn len(&self) -> usize {
        self.list.len()
    }

    fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    fn append(&mut self, other: &mut Self) {
        self.list.append(&mut other.list);
    }

    fn pop_list(&mut self) -> Option<Self> {
        if self.is_empty() {
            return None
        }

        let tail = self.list.split_off(1);

        Some(List {
            kind: None,
            list: mem::replace(&mut self.list, tail),
        })
    }

    /// de-initialize all nodes and gives them back to their inbox
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
        let inbox = Arc::new(AtomicCell::new(List::empty().into()));

        Queue {
            remote: Arc::new(AtomicCell::new(List::empty().into())),
            local: List::new(inbox.clone()),
            pocket: List::empty().into(),
            inbox,
        }
    }

    pub fn wait_or_notify(&mut self, kind: Kind, value: T) {
        debug_assert!(self.pocket.is_empty());

        self.local.init(kind, value);

        // fetch remote to pocket
        self.remote.swap(&mut self.pocket);

        loop {
            let mut to_notify = List::empty();

            // merge `self.local` to `self.pocket`
            match (self.local.kind, self.pocket.kind) {
                // contains different kind
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
                    let mut remote = mem::replace(&mut *self.pocket, remain);

                    to_notify.append(&mut self.local);
                    to_notify.append(&mut remote);
                }
                // both contain same kind or either one is empty
                _ => {
                    if self.pocket.kind.is_none() {
                        self.pocket.kind = self.local.kind;
                    }

                    self.pocket.append(&mut self.local);
                }
            }

            // push to remote
            self.remote.swap(&mut self.pocket);

            to_notify.notify_all(&mut self.pocket);

            // is remote changed since last fetch?
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
        let inbox = Arc::new(AtomicCell::new(List::empty().into()));

        Queue {
            remote: self.remote.clone(),
            local: List::new(inbox.clone()),
            pocket: List::empty().into(),
            inbox,
        }
    }
}
