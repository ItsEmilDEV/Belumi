use std::sync::{
    Arc, Mutex,
    atomic::{AtomicI32, Ordering},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Broadcast(String),
    Message { uuid: String, text: String },
    Kick { uuid: String, reason: String },
}

pub const MAX_QUEUED: usize = 1024;

#[derive(Clone, Default)]
pub struct Shared {
    queue: Arc<Mutex<Vec<Action>>>,
    players: Arc<AtomicI32>,
}

impl Shared {
    pub fn push(&self, a: Action) -> bool {
        let mut q = self.queue.lock().unwrap();
        if q.len() >= MAX_QUEUED {
            return false;
        }
        q.push(a);
        true
    }

    pub fn take(&self) -> Vec<Action> {
        std::mem::take(&mut *self.queue.lock().unwrap())
    }

    pub fn set_players(&self, n: usize) {
        self.players
            .store(i32::try_from(n).unwrap_or(i32::MAX), Ordering::Relaxed);
    }

    pub fn players(&self) -> i32 {
        self.players.load(Ordering::Relaxed)
    }
}
