use crate::error::CompatError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MinecraftVersion {
    V26_3,
    V26_2,
    V26_1,
    Unknown,
}

impl MinecraftVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V26_3 => "26.3",
            Self::V26_2 => "26.2",
            Self::V26_1 => "26.1",
            Self::Unknown => "unknown",
        }
    }

    pub fn data_version(&self) -> i32 {
        match self {
            Self::V26_3 => 5023,
            Self::V26_2 => 4903,
            Self::V26_1 => 4786,
            Self::Unknown => 0,
        }
    }

    pub fn is_native(&self) -> bool {
        matches!(self, Self::V26_3)
    }

    pub fn is_supported(&self) -> bool {
        matches!(self, Self::V26_3 | Self::V26_2 | Self::V26_1)
    }
}

impl std::fmt::Display for MinecraftVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProtocolVersion(i32);

impl ProtocolVersion {
    pub const V26_3: Self = Self(777);
    pub const V26_2: Self = Self(776);
    pub const V26_1: Self = Self(775);

    pub fn from_i32(value: i32) -> Self {
        Self(value)
    }

    pub fn as_i32(&self) -> i32 {
        self.0
    }

    pub fn minecraft_version(&self) -> MinecraftVersion {
        match self.0 {
            777 => MinecraftVersion::V26_3,
            776 => MinecraftVersion::V26_2,
            775 => MinecraftVersion::V26_1,
            _ => MinecraftVersion::Unknown,
        }
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetVersion {
    minecraft: MinecraftVersion,
    protocol: ProtocolVersion,
}

impl TargetVersion {
    pub fn from_minecraft(version: MinecraftVersion) -> Self {
        let protocol = match version {
            MinecraftVersion::V26_3 => ProtocolVersion::V26_3,
            MinecraftVersion::V26_2 => ProtocolVersion::V26_2,
            MinecraftVersion::V26_1 => ProtocolVersion::V26_1,
            MinecraftVersion::Unknown => ProtocolVersion::from_i32(0),
        };
        Self {
            minecraft: version,
            protocol,
        }
    }

    pub fn from_protocol(protocol: i32) -> Self {
        let mc_version = ProtocolVersion::from_i32(protocol).minecraft_version();
        Self {
            minecraft: mc_version,
            protocol: ProtocolVersion::from_i32(protocol),
        }
    }

    pub fn minecraft_version(&self) -> MinecraftVersion {
        self.minecraft
    }

    pub fn protocol_version(&self) -> ProtocolVersion {
        self.protocol
    }

    pub fn requires_translation(&self) -> bool {
        !self.minecraft.is_native()
    }

    pub fn is_supported(&self) -> bool {
        self.minecraft.is_supported()
    }

    pub fn validate(&self) -> Result<(), CompatError> {
        if !self.is_supported() {
            return Err(CompatError::UnsupportedVersion(
                self.minecraft.as_str().to_string(),
            ));
        }
        Ok(())
    }
}

impl std::fmt::Display for TargetVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (protocol {})", self.minecraft, self.protocol)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionInfo {
    pub native: MinecraftVersion,
    pub target: TargetVersion,
}

impl VersionInfo {
    pub fn new(target: TargetVersion) -> Self {
        Self {
            native: MinecraftVersion::V26_3,
            target,
        }
    }

    pub fn native() -> Self {
        Self {
            native: MinecraftVersion::V26_3,
            target: TargetVersion::from_minecraft(MinecraftVersion::V26_3),
        }
    }

    pub fn requires_translation(&self) -> bool {
        self.target.requires_translation()
    }
}

impl std::fmt::Display for VersionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native={}, target={}", self.native, self.target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_version_is_26_3() {
        assert_eq!(MinecraftVersion::V26_3.as_str(), "26.3");
        assert_eq!(MinecraftVersion::V26_3.data_version(), 5023);
        assert!(MinecraftVersion::V26_3.is_native());
        assert!(MinecraftVersion::V26_3.is_supported());
    }

    #[test]
    fn protocol_versions_are_correct() {
        assert_eq!(ProtocolVersion::V26_3.as_i32(), 777);
        assert_eq!(ProtocolVersion::V26_2.as_i32(), 776);
        assert_eq!(ProtocolVersion::V26_1.as_i32(), 775);
    }

    #[test]
    fn protocol_version_maps_to_minecraft_version() {
        assert_eq!(
            ProtocolVersion::from_i32(777).minecraft_version(),
            MinecraftVersion::V26_3
        );
        assert_eq!(
            ProtocolVersion::from_i32(776).minecraft_version(),
            MinecraftVersion::V26_2
        );
        assert_eq!(
            ProtocolVersion::from_i32(775).minecraft_version(),
            MinecraftVersion::V26_1
        );
        assert_eq!(
            ProtocolVersion::from_i32(999).minecraft_version(),
            MinecraftVersion::Unknown
        );
    }

    #[test]
    fn target_version_from_protocol() {
        let target = TargetVersion::from_protocol(776);
        assert_eq!(target.minecraft_version(), MinecraftVersion::V26_2);
        assert_eq!(target.protocol_version().as_i32(), 776);
        assert!(target.requires_translation());
        assert!(target.is_supported());
    }

    #[test]
    fn native_target_does_not_require_translation() {
        let target = TargetVersion::from_minecraft(MinecraftVersion::V26_3);
        assert!(!target.requires_translation());
    }

    #[test]
    fn version_info_native() {
        let info = VersionInfo::native();
        assert_eq!(info.native, MinecraftVersion::V26_3);
        assert!(!info.requires_translation());
    }

    #[test]
    fn version_info_with_target() {
        let target = TargetVersion::from_protocol(776);
        let info = VersionInfo::new(target);
        assert_eq!(info.native, MinecraftVersion::V26_3);
        assert!(info.requires_translation());
    }

    #[test]
    fn unsupported_version_rejected() {
        let target = TargetVersion::from_minecraft(MinecraftVersion::Unknown);
        assert!(!target.is_supported());
        assert!(target.validate().is_err());
    }

    #[test]
    fn supported_versions_accepted() {
        for version in [
            MinecraftVersion::V26_3,
            MinecraftVersion::V26_2,
            MinecraftVersion::V26_1,
        ] {
            let target = TargetVersion::from_minecraft(version);
            assert!(target.validate().is_ok());
        }
    }

    #[test]
    fn version_display() {
        let target = TargetVersion::from_protocol(776);
        let display = format!("{}", target);
        assert!(display.contains("26.2"));
        assert!(display.contains("776"));
    }

    #[test]
    fn data_versions_are_correct() {
        assert_eq!(MinecraftVersion::V26_3.data_version(), 5023);
        assert_eq!(MinecraftVersion::V26_2.data_version(), 4903);
        assert_eq!(MinecraftVersion::V26_1.data_version(), 4786);
    }
}
