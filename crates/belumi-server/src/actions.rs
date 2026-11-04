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
    roster: Arc<Mutex<Arc<String>>>,
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

    pub fn set_players(&self, list: &[(String, String)]) {
        self.players.store(
            i32::try_from(list.len()).unwrap_or(i32::MAX),
            Ordering::Relaxed,
        );
        let json: Vec<_> = list
            .iter()
            .map(|(uuid, name)| serde_json::json!({"uuid": uuid, "name": name}))
            .collect();
        *self.roster.lock().unwrap() = Arc::new(serde_json::Value::Array(json).to_string());
    }

    /// The online players as a JSON array of `{uuid, name}`.
    pub fn roster(&self) -> Arc<String> {
        self.roster.lock().unwrap().clone()
    }

    pub fn players(&self) -> i32 {
        self.players.load(Ordering::Relaxed)
    }
}
