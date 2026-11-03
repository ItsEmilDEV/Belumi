pub mod event;
pub mod manifest;

pub use event::{Event, EventKind, EventResult};
pub use manifest::{ManifestError, Permission, PluginManifest};

pub const ABI_VERSION: u32 = 1;
