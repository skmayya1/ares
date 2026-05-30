use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender};

use super::types::Event;

#[derive(Clone)]
pub struct EventSender {
    tx: Sender<Event>,
}

impl EventSender {
    pub fn send(&self, event: Event) {
        let _ = self.tx.send(event);
    }
}

pub struct EventBus {
    queue: VecDeque<Event>,
    external_rx: Receiver<Event>,
}

impl EventBus {
    pub fn new() -> (Self, EventSender) {
        let (tx, rx) = mpsc::channel();
        let bus = Self {
            queue: VecDeque::new(),
            external_rx: rx,
        };
        (bus, EventSender { tx })
    }

    pub fn push(&mut self, event: Event) {
        self.queue.push_back(event);
    }

    pub fn pop(&mut self) -> Option<Event> {
        self.queue.pop_front()
    }

    pub fn drain_external(&mut self) {
        while let Ok(event) = self.external_rx.try_recv() {
            self.queue.push_back(event);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
