use yaxi::proto::Event;

use proto::Sequence;

use std::sync::{Arc, Mutex, Condvar};
use std::collections::VecDeque;

macro_rules! lock {
    ($mutex:expr) => {
        $mutex.lock().map_err(|_| Into::<Box<dyn std::error::Error>>::into("failed to lock"))
    }
}

pub enum EventType {
    XEvent(Event),
    Config(Sequence),
}

#[derive(Clone)]
pub struct EventQueue {
    events: Arc<Mutex<VecDeque<EventType>>>,
    cond: Arc<Condvar>,
}

impl EventQueue {
    pub fn new() -> EventQueue {
        EventQueue {
            events: Arc::new(Mutex::new(VecDeque::new())),
            cond: Arc::new(Condvar::new()),
        }
    }

    pub fn push(&self, event: EventType) -> Result<(), Box<dyn std::error::Error>> {
        lock!(self.events)?.push_back(event);

        self.cond.notify_all();

        Ok(())
    }

    pub fn extend(&self, events: Vec<EventType>) -> Result<(), Box<dyn std::error::Error>> {
        lock!(self.events)?.extend(events);

        self.cond.notify_all();

        Ok(())
    }

    pub fn wait(&self) -> Result<EventType, Box<dyn std::error::Error>> {
        let mut guard = lock!(self.events)?;

        loop {
            if let Some(event) = guard.pop_back() {
                return Ok(event);
            } else {
                guard = self.cond.wait(guard).map_err(|_| Into::<Box<dyn std::error::Error>>::into("failed to lock"))?;
            }
        }
    }
}
