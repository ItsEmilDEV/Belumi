mod actions;
pub mod config;
mod manager;
mod plugin;

pub use actions::{Action, MAX_QUEUED};
pub use manager::PluginManager;
pub use plugin::{Plugin, PluginError};
