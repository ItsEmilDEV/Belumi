use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompatError {
    #[error("unsupported target version: {0}")]
    UnsupportedVersion(String),

    #[error("protocol version mismatch: expected {expected}, got {actual}")]
    ProtocolMismatch { expected: i32, actual: i32 },

    #[error("packet translation failed: {0}")]
    PacketTranslation(String),

    #[error("registry mapping failed for '{registry}': {reason}")]
    RegistryMapping { registry: String, reason: String },

    #[error("item component translation failed: {0}")]
    ItemComponent(String),

    #[error("entity data translation failed: {0}")]
    EntityData(String),

    #[error("version detection failed: {0}")]
    VersionDetection(String),

    #[error("chunk format incompatibility: {0}")]
    ChunkFormat(String),
}

pub type CompatResult<T> = Result<T, CompatError>;
