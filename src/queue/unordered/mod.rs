
use std::sync::Arc;
use std::collections::LinkedList;

use role::Kind;

mod atomic_cell;

use self::atomic_cell::AtomicCell;

#[derive(Debug)]
pub struct Queue<T> {
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

impl<T> Default for List<T> {
    fn default() -> Self {
        List {
            kind: None,
            list: Default::default(),
        }
    }
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        let mut queue = Queue {
            remote: Arc::default(),
            local: List::default(),
            pocket: Box::default(),
            inbox: Arc::default(),
        };

        queue.local.list.push_front(Msg {
            value: None,
            inbox: Arc::clone(&queue.inbox),
        });

        queue
    }
}

impl<T> Clone for Queue<T> {
    fn clone(&self) -> Self {
        Queue {
            remote: Arc::clone(&self.remote),
            local: List::default(),
            pocket: Box::default(),
            inbox: Arc::default(),
        }
    }
}
