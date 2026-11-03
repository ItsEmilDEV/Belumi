use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    ServerStart,
    ServerStop,
    Tick,
    PlayerJoin,
    PlayerQuit,
    Chat,
    Command,
    BlockBreak,
    PluginCommand,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    ServerStart,
    ServerStop,
    Tick {
        tick: u64,
    },
    PlayerJoin {
        uuid: String,
        name: String,
    },
    PlayerQuit {
        uuid: String,
        name: String,
    },
    Chat {
        uuid: String,
        message: String,
    },
    Command {
        uuid: String,
        command: String,
    },
    BlockBreak {
        uuid: String,
        dimension: String,
        x: i32,
        y: i32,
        z: i32,
    },
    PluginCommand {
        uuid: String,
        name: String,
        args: String,
    },
}

impl Event {
    pub fn kind(&self) -> EventKind {
        match self {
            Event::ServerStart => EventKind::ServerStart,
            Event::ServerStop => EventKind::ServerStop,
            Event::Tick { .. } => EventKind::Tick,
            Event::PlayerJoin { .. } => EventKind::PlayerJoin,
            Event::PlayerQuit { .. } => EventKind::PlayerQuit,
            Event::Chat { .. } => EventKind::Chat,
            Event::Command { .. } => EventKind::Command,
            Event::BlockBreak { .. } => EventKind::BlockBreak,
            Event::PluginCommand { .. } => EventKind::PluginCommand,
        }
    }

    pub fn cancellable(&self) -> bool {
        matches!(
            self.kind(),
            EventKind::Chat | EventKind::Command | EventKind::BlockBreak
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventResult {
    #[default]
    Continue,
    Cancel,
}
